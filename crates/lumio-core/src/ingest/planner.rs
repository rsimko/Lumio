use std::collections::HashSet;
use std::path::{Path, PathBuf};

use chrono::Local;
use natord::compare_ignore_case;
use uuid::Uuid;

use crate::error::{LumioError, Result};
use crate::ingest::association::{SourceEntity, scan_sources};
use crate::ingest::metadata::{
    capture_for_entity, detect_tools, read_exif_dates_for_sources, validate_destination,
};
use crate::ingest::template::{
    TemplateContext, TemplateKind, render_folder_segments, render_text, validate_rendered_filename,
    validate_required_inputs,
};
use crate::ingest::types::{
    CaptureInfo, IngestConflict, IngestConflictResolution, IngestEntity, IngestEntityStatus,
    IngestFileRole, IngestIptcValues, IngestLogEntry, IngestLogLevel, IngestMetadataTarget,
    IngestMetadataTargetKind, IngestPlan, IngestPlanSummary, IngestPlannedFile, IngestRequest,
    IngestRules, IngestWarning, XmpNaming,
};
use crate::path_utils::now_ms;

fn log_entry(
    level: IngestLogLevel,
    message: impl Into<String>,
    path: Option<PathBuf>,
    entity_id: Option<String>,
) -> IngestLogEntry {
    IngestLogEntry {
        at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        level,
        message: message.into(),
        path,
        entity_id,
    }
}

fn extension_of(path: &Path) -> String {
    path.extension()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn added_file_name(extension: &str) -> String {
    if extension.is_empty() {
        String::new()
    } else {
        format!(".{extension}")
    }
}

fn photo_destination(stem: &str, source: &Path) -> String {
    format!("{stem}{}", added_file_name(&extension_of(source)))
}

fn xmp_base_name(stem: &str, primary_extension: &str, naming: XmpNaming) -> String {
    match naming {
        XmpNaming::FilenameExt => format!("{stem}{}", added_file_name(primary_extension)),
        XmpNaming::Filename => stem.to_string(),
    }
}

fn ordered_xmps(entity: &IngestEntity) -> Vec<&crate::ingest::types::IngestSourceFile> {
    let mut xmps: Vec<_> = entity
        .files
        .iter()
        .filter(|file| file.role == IngestFileRole::Xmp)
        .collect();
    let primary_name = entity
        .primary_path
        .file_name()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let stem = entity.stem.to_ascii_lowercase();
    xmps.sort_by(|left, right| {
        let left_base = left
            .path
            .file_stem()
            .map(|value| value.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let right_base = right
            .path
            .file_stem()
            .map(|value| value.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let left_rank = if left_base == primary_name {
            0
        } else if left_base == stem {
            1
        } else {
            2
        };
        let right_rank = if right_base == primary_name {
            0
        } else if right_base == stem {
            1
        } else {
            2
        };
        left_rank
            .cmp(&right_rank)
            .then_with(|| compare_ignore_case(&left_base, &right_base))
    });
    xmps
}

fn planned_file(
    source: Option<PathBuf>,
    destination: PathBuf,
    role: IngestFileRole,
    size: u64,
    primary: bool,
    generated: bool,
) -> IngestPlannedFile {
    IngestPlannedFile {
        source,
        destination,
        role,
        size,
        primary,
        generated,
    }
}

fn build_planned_files(
    entity: &IngestEntity,
    rules: &IngestRules,
    stem: &str,
) -> Vec<IngestPlannedFile> {
    let directory = &entity.base_destination_dir;
    let primary_extension = extension_of(&entity.primary_path);
    let mut planned = Vec::new();

    for file in entity.files.iter().filter(|file| file.role.is_photo()) {
        planned.push(planned_file(
            Some(file.path.clone()),
            directory.join(photo_destination(stem, &file.path)),
            file.role,
            file.size,
            file.path == entity.primary_path,
            false,
        ));
    }

    let xmps = ordered_xmps(entity);
    let xmp_base = xmp_base_name(stem, &primary_extension, rules.xmp_naming);
    if !xmps.is_empty() {
        let canonical = directory.join(format!("{xmp_base}.xmp"));
        planned.push(planned_file(
            Some(xmps[0].path.clone()),
            canonical,
            IngestFileRole::Xmp,
            xmps[0].size,
            false,
            false,
        ));
        for (index, extra) in xmps.iter().enumerate().skip(1) {
            planned.push(planned_file(
                Some(extra.path.clone()),
                directory.join(format!("{xmp_base}.extra{}.xmp", index + 1)),
                IngestFileRole::Xmp,
                extra.size,
                false,
                false,
            ));
        }
    } else if !entity.iptc.is_empty() && entity.primary_role == IngestFileRole::Raw {
        planned.push(planned_file(
            None,
            directory.join(format!("{xmp_base}.xmp")),
            IngestFileRole::Xmp,
            0,
            false,
            true,
        ));
    }

    planned
}

fn metadata_target(
    entity: &IngestEntity,
    rules: &IngestRules,
    stem: &str,
    planned: &[IngestPlannedFile],
) -> Option<IngestMetadataTarget> {
    if entity.primary_role == IngestFileRole::Raw {
        if entity.iptc.is_empty() {
            return None;
        }
        let xmp_base = xmp_base_name(stem, &extension_of(&entity.primary_path), rules.xmp_naming);
        return Some(IngestMetadataTarget {
            path: entity.base_destination_dir.join(format!("{xmp_base}.xmp")),
            kind: IngestMetadataTargetKind::XmpSidecar,
        });
    }
    if entity.primary_role == IngestFileRole::Jpeg && !entity.iptc.is_empty() {
        return planned
            .iter()
            .find(|file| file.primary)
            .map(|file| IngestMetadataTarget {
                path: file.destination.clone(),
                kind: IngestMetadataTargetKind::Jpeg,
            });
    }
    None
}

fn path_key(path: &Path) -> String {
    path.to_string_lossy().to_ascii_lowercase()
}

fn existing_paths(files: &[IngestPlannedFile]) -> Vec<PathBuf> {
    files
        .iter()
        .map(|file| file.destination.clone())
        .filter(|path| path.exists())
        .collect()
}

fn conflict_message(
    existing: usize,
    resolution: IngestConflictResolution,
    renamed_to: Option<&str>,
) -> String {
    match resolution {
        IngestConflictResolution::Skip => format!(
            "{existing} destination file{} already exist{}. This photo will be skipped unless you choose Keep both or Replace.",
            if existing == 1 { "" } else { "s" },
            if existing == 1 { "s" } else { "" }
        ),
        IngestConflictResolution::KeepBoth => match renamed_to {
            Some(name) => format!(
                "{existing} destination file{} already exist{}. Keeping both by writing this photo as {name}.",
                if existing == 1 { "" } else { "s" },
                if existing == 1 { "s" } else { "" }
            ),
            None => format!(
                "{existing} destination file{} already exist{}. Existing files will be kept.",
                if existing == 1 { "" } else { "s" },
                if existing == 1 { "s" } else { "" }
            ),
        },
        IngestConflictResolution::Replace => format!(
            "{existing} destination file{} will be replaced after an explicit confirmation.",
            if existing == 1 { "" } else { "s" },
        ),
    }
}

fn assign_paths(plan: &mut IngestPlan) {
    let mut reserved: HashSet<String> = HashSet::new();
    let rules = plan.rules.clone();

    for entity in &mut plan.entities {
        if entity.status == IngestEntityStatus::Error {
            continue;
        }
        let base_files = build_planned_files(entity, &rules, &entity.base_destination_stem);
        let base_existing = existing_paths(&base_files);
        let has_existing = !base_existing.is_empty();

        let mut suffix = 1u32;
        let mut stem = entity.base_destination_stem.clone();
        let mut rename_reason = None;
        loop {
            entity.destination_dir = entity.base_destination_dir.clone();
            entity.destination_stem = stem.clone();
            let files = build_planned_files(entity, &rules, &stem);
            let keys: Vec<String> = files
                .iter()
                .map(|file| path_key(&file.destination))
                .collect();
            let duplicate = keys.iter().any(|key| reserved.contains(key));
            let existing = existing_paths(&files);
            let keep_both_blocked =
                entity.resolution == IngestConflictResolution::KeepBoth && !existing.is_empty();
            // Once a suffix has been chosen, never land on an existing file: the
            // suffix exists to avoid overwriting. Only the original name may
            // intentionally conflict (Skip or Replace).
            let renamed_onto_existing = suffix > 1 && !existing.is_empty();
            if duplicate || keep_both_blocked || renamed_onto_existing {
                suffix += 1;
                stem = format!("{}_{}", entity.base_destination_stem, suffix);
                if rename_reason.is_none() {
                    rename_reason = Some(if !existing.is_empty() {
                        "The destination already contains this name; keeping both.".to_string()
                    } else {
                        "Another photo in this ingest would use the same destination name."
                            .to_string()
                    });
                }
                continue;
            }

            let skipped = has_existing && entity.resolution == IngestConflictResolution::Skip;
            entity.status = if skipped {
                IngestEntityStatus::Conflict
            } else {
                IngestEntityStatus::Ready
            };
            entity.conflict = if has_existing {
                Some(IngestConflict {
                    existing: base_existing.clone(),
                    message: conflict_message(
                        base_existing.len(),
                        entity.resolution,
                        if suffix > 1 { Some(&stem) } else { None },
                    ),
                })
            } else {
                None
            };
            entity.renamed = suffix > 1;
            entity.rename_reason = rename_reason.clone();
            entity.planned_files = files;
            entity.metadata_target = metadata_target(entity, &rules, &stem, &entity.planned_files);
            if !skipped {
                for key in keys {
                    reserved.insert(key);
                }
            }
            break;
        }
    }
}

fn rebuild_summary(plan: &mut IngestPlan) {
    let mut summary = IngestPlanSummary {
        entities: plan.entities.len() as u32,
        warnings: plan.warnings.len() as u32,
        ..Default::default()
    };
    for entity in &plan.entities {
        if entity.status == IngestEntityStatus::Error {
            summary.errors += 1;
            continue;
        }
        summary.files += entity.planned_files.len() as u32;
        summary.bytes += entity
            .planned_files
            .iter()
            .map(|file| file.size)
            .sum::<u64>();
        if entity.conflict.is_some() {
            summary.conflicts += 1;
        }
        if entity.renamed {
            summary.renamed += 1;
        }
    }
    plan.summary = summary;
}

fn literal_optional(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

/// IPTC fields are literal text. They are never parsed as templates, so braces
/// and other characters in a title or keyword reach the destination verbatim.
fn literal_iptc(rules: &IngestRules) -> IngestIptcValues {
    let mut keywords = Vec::new();
    let mut seen = HashSet::new();
    for keyword in &rules.iptc.keywords {
        let keyword = keyword.trim();
        if !keyword.is_empty() && seen.insert(keyword.to_ascii_lowercase()) {
            keywords.push(keyword.to_string());
        }
    }
    IngestIptcValues {
        title: literal_optional(&rules.iptc.title),
        description: literal_optional(&rules.iptc.description),
        creator: literal_optional(&rules.iptc.creator),
        copyright: literal_optional(&rules.iptc.copyright),
        keywords,
    }
}

fn validate_rules(rules: &IngestRules) -> Vec<String> {
    let mut problems = validate_required_inputs(
        &rules.folder_template,
        TemplateKind::Folder,
        &rules.job_code,
        &rules.custom_text,
    );
    problems.extend(validate_required_inputs(
        &rules.filename_template,
        TemplateKind::Filename,
        &rules.job_code,
        &rules.custom_text,
    ));
    problems.sort();
    problems.dedup();
    problems
}

struct Prepared {
    source: SourceEntity,
    capture: CaptureInfo,
    iptc: IngestIptcValues,
    warnings: Vec<String>,
}

pub fn plan_ingest(request: &IngestRequest) -> Result<IngestPlan> {
    let problems = validate_rules(&request.rules);
    if !problems.is_empty() {
        return Err(LumioError::Ingest(problems.join(" ")));
    }
    let destination = validate_destination(&request.destination, &request.sources)?;
    let tools = detect_tools(request.exiftool_path.as_deref());
    if !request.rules.iptc.is_empty() && tools.exiftool.is_none() {
        return Err(LumioError::Ingest(
            "IPTC and keyword writing requires ExifTool. Install ExifTool or set its path in Settings → Ingest."
                .into(),
        ));
    }

    let mut plan = IngestPlan {
        id: Uuid::new_v4().to_string(),
        created_at: now_ms(),
        sources: request.sources.clone(),
        destination: destination.clone(),
        rules: request.rules.clone(),
        exiftool_path: tools.exiftool.as_ref().map(|info| info.path.clone()),
        tools: tools.clone(),
        entities: Vec::new(),
        warnings: Vec::new(),
        log: Vec::new(),
        summary: IngestPlanSummary::default(),
    };

    plan.log.push(log_entry(
        IngestLogLevel::Info,
        format!(
            "Scanning {} source{} for RAW, JPEG and XMP files.",
            request.sources.len(),
            if request.sources.len() == 1 { "" } else { "s" }
        ),
        None,
        None,
    ));
    let (sources, association_warnings) = scan_sources(&request.sources)?;
    for warning in association_warnings {
        plan.log.push(log_entry(
            IngestLogLevel::Warning,
            warning.message.clone(),
            warning.path.clone(),
            warning.entity_id.clone(),
        ));
        plan.warnings.push(warning);
    }
    plan.log.push(log_entry(
        IngestLogLevel::Info,
        format!(
            "Associated {} file{} into {} photo entit{}.",
            sources
                .iter()
                .map(|source| source.files.len())
                .sum::<usize>(),
            if sources
                .iter()
                .map(|source| source.files.len())
                .sum::<usize>()
                == 1
            {
                ""
            } else {
                "s"
            },
            sources.len(),
            if sources.len() == 1 { "y" } else { "ies" }
        ),
        None,
        None,
    ));

    if let Some(info) = &tools.exiftool {
        plan.log.push(log_entry(
            IngestLogLevel::Info,
            format!(
                "Metadata reader: ExifTool {} at {} (with built-in RAW/JPEG fallback).",
                info.version,
                info.path.display()
            ),
            Some(info.path.clone()),
            None,
        ));
    } else {
        plan.log.push(log_entry(
            IngestLogLevel::Info,
            "Metadata reader: built-in RAW/JPEG reader.",
            None,
            None,
        ));
    }

    let all_files: Vec<_> = sources
        .iter()
        .flat_map(|source| source.files.iter().cloned())
        .collect();
    let exif = read_exif_dates_for_sources(&all_files, plan.exiftool_path.as_deref());
    for warning in exif.warnings {
        plan.log.push(log_entry(
            IngestLogLevel::Warning,
            warning.clone(),
            None,
            None,
        ));
        plan.warnings.push(IngestWarning {
            path: None,
            entity_id: None,
            message: warning,
        });
    }

    let mut prepared = Vec::new();
    let iptc = literal_iptc(&request.rules);
    for source in sources {
        let capture = capture_for_entity(
            &source.files,
            source
                .files
                .iter()
                .find(|file| file.path == source.primary_path)
                .map(|file| file.modified_ms)
                .unwrap_or(0),
            &exif.dates,
        );
        let mut warnings = Vec::new();
        if let Some(warning) = &capture.warning {
            warnings.push(warning.clone());
            plan.warnings.push(IngestWarning {
                path: Some(source.primary_path.clone()),
                entity_id: Some(source.id.clone()),
                message: warning.clone(),
            });
            plan.log.push(log_entry(
                IngestLogLevel::Warning,
                warning.clone(),
                Some(source.primary_path.clone()),
                Some(source.id.clone()),
            ));
        }
        prepared.push(Prepared {
            source,
            capture,
            iptc: iptc.clone(),
            warnings,
        });
    }

    prepared.sort_by(|left, right| {
        left.capture
            .timestamp_ms
            .cmp(&right.capture.timestamp_ms)
            .then_with(|| {
                compare_ignore_case(
                    &left.source.source_dir.to_string_lossy(),
                    &right.source.source_dir.to_string_lossy(),
                )
            })
            .then_with(|| compare_ignore_case(&left.source.stem, &right.source.stem))
            .then_with(|| left.source.id.cmp(&right.source.id))
    });

    let counter_start = request.rules.counter_start;
    let counter_padding = request.rules.counter_padding.min(12);
    for (index, prepared) in prepared.into_iter().enumerate() {
        let counter = counter_start.saturating_add(index as u32);
        let context = TemplateContext {
            capture: &prepared.capture,
            job_code: &request.rules.job_code,
            custom_text: &request.rules.custom_text,
            default_filename: &prepared.source.stem,
            counter,
            counter_padding,
        };
        let folder = render_folder_segments(&request.rules.folder_template, &context)
            .map_err(LumioError::Ingest)?;
        let stem = render_text(&request.rules.filename_template, &context)
            .and_then(|value| validate_rendered_filename(&value))
            .map_err(LumioError::Ingest)?;
        let destination_dir = folder
            .iter()
            .fold(destination.clone(), |path, segment| path.join(segment));

        let mut entity = IngestEntity {
            id: prepared.source.id.clone(),
            source_dir: prepared.source.source_dir.clone(),
            stem: prepared.source.stem.clone(),
            display_name: prepared
                .source
                .primary_path
                .file_name()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| prepared.source.stem.clone()),
            primary_path: prepared.source.primary_path.clone(),
            primary_role: prepared.source.primary_role,
            files: prepared.source.files.clone(),
            capture: prepared.capture.clone(),
            base_destination_dir: destination_dir.clone(),
            base_destination_stem: stem.clone(),
            destination_dir,
            destination_stem: stem,
            planned_files: Vec::new(),
            metadata_target: None,
            status: IngestEntityStatus::Ready,
            resolution: IngestConflictResolution::Skip,
            conflict: None,
            renamed: false,
            rename_reason: None,
            warnings: prepared.warnings,
            iptc: prepared.iptc,
        };
        entity.planned_files =
            build_planned_files(&entity, &request.rules, &entity.base_destination_stem);
        entity.metadata_target = metadata_target(
            &entity,
            &request.rules,
            &entity.base_destination_stem,
            &entity.planned_files,
        );
        for planned in &entity.planned_files {
            plan.log.push(log_entry(
                IngestLogLevel::Info,
                format!(
                    "{} → {}{}",
                    planned
                        .source
                        .as_ref()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| "(generated XMP)".into()),
                    planned.destination.display(),
                    if planned.generated {
                        " (generated)"
                    } else {
                        ""
                    }
                ),
                Some(planned.destination.clone()),
                Some(entity.id.clone()),
            ));
        }
        plan.entities.push(entity);
    }

    assign_paths(&mut plan);
    rebuild_summary(&mut plan);
    for entity in &plan.entities {
        if entity.status == IngestEntityStatus::Error {
            continue;
        }
        if let Some(conflict) = &entity.conflict {
            plan.log.push(log_entry(
                IngestLogLevel::Warning,
                conflict.message.clone(),
                Some(entity.destination_dir.clone()),
                Some(entity.id.clone()),
            ));
        }
        for warning in &entity.warnings {
            if !plan.warnings.iter().any(|existing| {
                existing.entity_id.as_deref() == Some(entity.id.as_str())
                    && existing.message == *warning
            }) {
                plan.warnings.push(IngestWarning {
                    path: Some(entity.primary_path.clone()),
                    entity_id: Some(entity.id.clone()),
                    message: warning.clone(),
                });
            }
        }
    }
    plan.summary.warnings = plan.warnings.len() as u32;
    plan.log.push(log_entry(
        IngestLogLevel::Info,
        format!(
            "Dry run complete: {} entit{}, {} file{}, {} conflict{}, {} error{}.",
            plan.summary.entities,
            if plan.summary.entities == 1 {
                "y"
            } else {
                "ies"
            },
            plan.summary.files,
            if plan.summary.files == 1 { "" } else { "s" },
            plan.summary.conflicts,
            if plan.summary.conflicts == 1 { "" } else { "s" },
            plan.summary.errors,
            if plan.summary.errors == 1 { "" } else { "s" }
        ),
        Some(destination),
        None,
    ));
    Ok(plan)
}

/// Recompute destination names and conflicts from the stored plan. Templates are
/// never rendered again, so this is still exactly the plan the preview showed.
pub fn rebuild_plan_paths(plan: &mut IngestPlan) {
    assign_paths(plan);
    rebuild_summary(plan);
    plan.summary.warnings = plan.warnings.len() as u32;
}

pub fn set_entity_resolution(
    plan: &mut IngestPlan,
    entity_id: &str,
    resolution: IngestConflictResolution,
) -> Result<()> {
    let entity = plan
        .entities
        .iter_mut()
        .find(|entity| entity.id == entity_id)
        .ok_or_else(|| LumioError::Ingest("The planned photo no longer exists.".into()))?;
    entity.resolution = resolution;
    let name = entity.display_name.clone();
    rebuild_plan_paths(plan);
    plan.log.push(log_entry(
        IngestLogLevel::Info,
        format!(
            "Conflict resolution for {name}: {}.",
            match resolution {
                IngestConflictResolution::Skip => "skip",
                IngestConflictResolution::KeepBoth => "keep both",
                IngestConflictResolution::Replace => "replace",
            }
        ),
        None,
        Some(entity_id.to_string()),
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::types::XmpNaming;
    use std::fs;

    fn rules() -> IngestRules {
        IngestRules {
            folder_template: "{YYYY}/{MM}".into(),
            filename_template: "{YYYY}{MM}{DD}_{default_filename}".into(),
            job_code: "JOB".into(),
            custom_text: "Text".into(),
            counter_start: 1,
            counter_padding: 4,
            xmp_naming: XmpNaming::FilenameExt,
            iptc: Default::default(),
        }
    }

    fn request(sources: Vec<PathBuf>, destination: &Path) -> IngestRequest {
        IngestRequest {
            sources,
            destination: destination.to_path_buf(),
            rules: rules(),
            exiftool_path: None,
        }
    }

    #[test]
    fn dry_run_generates_paths_with_one_counter_per_entity() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(source.join("a.CR3"), b"raw").unwrap();
        fs::write(source.join("a.jpg"), b"jpeg").unwrap();
        fs::write(source.join("a.CR3.xmp"), b"<xmp/>").unwrap();
        fs::write(source.join("b.CR3"), b"raw").unwrap();

        let plan = plan_ingest(&request(vec![source], &destination)).unwrap();
        assert_eq!(plan.entities.len(), 2);
        assert_eq!(plan.summary.entities, 2);
        assert_eq!(plan.summary.files, 4);
        assert_eq!(plan.entities[0].planned_files.len(), 3);
        assert_eq!(plan.entities[1].planned_files.len(), 1);
        assert!(
            plan.log
                .iter()
                .any(|entry| entry.message.contains("Dry run complete"))
        );
        // Counters are rendered into the filename in capture-time order. Both
        // files fall back to modified time here, so the template keeps the
        // original names distinct.
        let names: HashSet<String> = plan
            .entities
            .iter()
            .map(|entity| entity.destination_stem.clone())
            .collect();
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn duplicate_output_names_are_auto_renamed() {
        let temp = tempfile::tempdir().unwrap();
        let source_a = temp.path().join("a");
        let source_b = temp.path().join("b");
        let destination = temp.path().join("destination");
        fs::create_dir_all(&source_a).unwrap();
        fs::create_dir_all(&source_b).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(source_a.join("same.CR3"), b"raw").unwrap();
        fs::write(source_b.join("same.CR3"), b"raw").unwrap();

        let mut request = request(vec![source_a, source_b], &destination);
        request.rules.filename_template = "{default_filename}".into();
        let plan = plan_ingest(&request).unwrap();
        assert_eq!(plan.entities.len(), 2);
        let mut stems: Vec<String> = plan
            .entities
            .iter()
            .map(|entity| entity.destination_stem.clone())
            .collect();
        stems.sort();
        assert_eq!(stems, vec!["same", "same_2"]);
        assert!(plan.entities.iter().any(|entity| entity.renamed));
    }

    #[test]
    fn auto_rename_skips_existing_suffixed_files() {
        let temp = tempfile::tempdir().unwrap();
        let source_a = temp.path().join("a");
        let source_b = temp.path().join("b");
        let destination = temp.path().join("destination");
        fs::create_dir_all(&source_a).unwrap();
        fs::create_dir_all(&source_b).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(source_a.join("same.CR3"), b"one").unwrap();
        fs::write(source_b.join("same.CR3"), b"two").unwrap();
        fs::write(destination.join("same_2.CR3"), b"existing").unwrap();

        let mut request = request(vec![source_a, source_b], &destination);
        request.rules.folder_template = String::new();
        request.rules.filename_template = "{default_filename}".into();
        let plan = plan_ingest(&request).unwrap();
        assert_eq!(plan.entities.len(), 2);
        let mut stems: Vec<String> = plan
            .entities
            .iter()
            .map(|entity| entity.destination_stem.clone())
            .collect();
        stems.sort();
        assert_eq!(stems, vec!["same", "same_3"]);
        assert_eq!(
            fs::read(destination.join("same_2.CR3")).unwrap(),
            b"existing"
        );
    }

    #[test]
    fn existing_destination_is_a_conflict_and_skip_reserves_the_plan() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(source.join("photo.CR3"), b"raw").unwrap();
        let mut request = request(vec![source], &destination);
        request.rules.filename_template = "{default_filename}".into();
        let plan = plan_ingest(&request).unwrap();
        let target = plan.entities[0].planned_files[0].destination.clone();
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, b"existing").unwrap();

        let plan = plan_ingest(&request).unwrap();
        assert_eq!(plan.entities[0].status, IngestEntityStatus::Conflict);
        assert_eq!(plan.entities[0].resolution, IngestConflictResolution::Skip);
        assert_eq!(plan.summary.conflicts, 1);
    }

    #[test]
    fn keep_both_renames_the_whole_entity() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(source.join("photo.CR3"), b"raw").unwrap();
        fs::write(source.join("photo.jpg"), b"jpeg").unwrap();
        let mut request = request(vec![source], &destination);
        request.rules.filename_template = "{default_filename}".into();
        let plan = plan_ingest(&request).unwrap();
        let target = plan.entities[0].planned_files[0].destination.clone();
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, b"existing").unwrap();

        let mut plan = plan_ingest(&request).unwrap();
        let id = plan.entities[0].id.clone();
        set_entity_resolution(&mut plan, &id, IngestConflictResolution::KeepBoth).unwrap();
        let entity = &plan.entities[0];
        assert_eq!(entity.status, IngestEntityStatus::Ready);
        assert_eq!(entity.destination_stem, "photo_2");
        assert!(
            entity
                .planned_files
                .iter()
                .all(|file| file.destination.to_string_lossy().contains("photo_2"))
        );
        assert!(entity.metadata_target.is_none());
    }

    #[test]
    fn replace_keeps_paths_and_marks_ready() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(source.join("photo.CR3"), b"raw").unwrap();
        let mut request = request(vec![source], &destination);
        request.rules.filename_template = "{default_filename}".into();
        let plan = plan_ingest(&request).unwrap();
        let target = plan.entities[0].planned_files[0].destination.clone();
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, b"existing").unwrap();

        let mut plan = plan_ingest(&request).unwrap();
        let id = plan.entities[0].id.clone();
        set_entity_resolution(&mut plan, &id, IngestConflictResolution::Replace).unwrap();
        let entity = &plan.entities[0];
        assert_eq!(entity.status, IngestEntityStatus::Ready);
        assert_eq!(entity.destination_stem, "photo");
        assert_eq!(entity.resolution, IngestConflictResolution::Replace);
    }

    #[test]
    fn invalid_templates_are_rejected_before_scanning() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        fs::create_dir_all(&source).unwrap();
        let mut request = request(vec![source], temp.path());
        request.rules.filename_template = "{me}.jpg".into();
        let error = plan_ingest(&request).unwrap_err().to_string();
        assert!(error.contains("Unknown template token") || error.contains("camera token"));
    }

    #[test]
    fn xmp_naming_setting_changes_the_sidecar_name() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(source.join("photo.CR3"), b"raw").unwrap();
        fs::write(source.join("photo.CR3.xmp"), b"<xmp/>").unwrap();
        let mut request = request(vec![source], &destination);
        request.rules.filename_template = "NEW".into();
        let plan = plan_ingest(&request).unwrap();
        let xmp = plan.entities[0]
            .planned_files
            .iter()
            .find(|file| file.role == IngestFileRole::Xmp)
            .unwrap();
        assert!(xmp.destination.ends_with("NEW.CR3.xmp"));

        request.rules.xmp_naming = XmpNaming::Filename;
        let plan = plan_ingest(&request).unwrap();
        let xmp = plan.entities[0]
            .planned_files
            .iter()
            .find(|file| file.role == IngestFileRole::Xmp)
            .unwrap();
        assert!(xmp.destination.ends_with("NEW.xmp"));
    }

    #[test]
    fn iptc_values_are_literal_text_and_are_deduplicated() {
        let mut rules = rules();
        rules.iptc.title = "{YYYY} {job_code}".into();
        rules.iptc.description = "  ".into();
        rules.iptc.creator = "Jane Doe".into();
        rules.iptc.copyright = "© 2026 Jane".into();
        rules.iptc.keywords = vec![
            "Travel".into(),
            "travel".into(),
            " {counter} ".into(),
            "  ".into(),
        ];

        let iptc = literal_iptc(&rules);
        assert_eq!(iptc.title.as_deref(), Some("{YYYY} {job_code}"));
        assert_eq!(iptc.description, None);
        assert_eq!(iptc.creator.as_deref(), Some("Jane Doe"));
        assert_eq!(iptc.copyright.as_deref(), Some("© 2026 Jane"));
        assert_eq!(iptc.keywords, vec!["Travel", "{counter}"]);
    }
}
