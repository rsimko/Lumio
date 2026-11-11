use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use chrono::Local;

use crate::error::{LumioError, Result};
use crate::ingest::iptc::{create_empty_xmp, write_iptc};
use crate::ingest::types::{
    IngestConflictResolution, IngestEntity, IngestEntityResult, IngestEntityStatus, IngestEvent,
    IngestLogEntry, IngestLogLevel, IngestOutcome, IngestPlan, IngestResultStatus,
};
use crate::path_utils::display_name;

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

fn hash_file(path: &Path) -> Result<blake3::Hash> {
    let mut file = fs::File::open(path).map_err(|error| LumioError::io(path, error))?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| LumioError::io(path, error))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize())
}

/// Copy one file and prove the destination is byte-for-byte identical before it
/// can be committed. Sources are opened read-only.
fn copy_and_verify(source: &Path, destination: &Path) -> Result<u64> {
    let copied =
        fs::copy(source, destination).map_err(|error| LumioError::io(destination, error))?;
    let source_size = fs::metadata(source)
        .map_err(|error| LumioError::io(source, error))?
        .len();
    let destination_size = fs::metadata(destination)
        .map_err(|error| LumioError::io(destination, error))?
        .len();
    if copied != source_size || destination_size != source_size {
        let _ = fs::remove_file(destination);
        return Err(LumioError::Ingest(format!(
            "The copied size does not match the original for {}.",
            source.display()
        )));
    }
    let source_hash = hash_file(source)?;
    let destination_hash = hash_file(destination)?;
    if source_hash != destination_hash {
        let _ = fs::remove_file(destination);
        return Err(LumioError::Ingest(format!(
            "The copy verification failed for {}.",
            source.display()
        )));
    }
    Ok(copied)
}

struct EntityRun {
    log: Vec<IngestLogEntry>,
    result: IngestEntityResult,
}

struct EntityContext<'a> {
    plan: &'a IngestPlan,
    temp_root: &'a Path,
    index: usize,
}

fn execute_entity<F>(
    context: &EntityContext<'_>,
    entity: &IngestEntity,
    cancel: &AtomicBool,
    emit: &mut F,
) -> Result<EntityRun>
where
    F: FnMut(IngestEvent),
{
    let mut run = EntityRun {
        log: Vec::new(),
        result: IngestEntityResult {
            entity_id: entity.id.clone(),
            display_name: entity.display_name.clone(),
            status: IngestResultStatus::Failed,
            files: 0,
            bytes: 0,
            destination: Some(entity.destination_dir.clone()),
            message: None,
        },
    };

    if entity.planned_files.is_empty() {
        run.result.status = IngestResultStatus::Skipped;
        run.result.message = Some("No files were planned for this photo.".into());
        return Ok(run);
    }

    // Re-check the destination right before copying. A file that appeared after
    // the dry run is a hard failure unless the user explicitly chose Replace.
    for file in &entity.planned_files {
        if file.destination.exists() {
            if entity.resolution != IngestConflictResolution::Replace {
                return Err(LumioError::Ingest(format!(
                    "{} appeared after the dry run. Run the preview again before ingesting.",
                    file.destination.display()
                )));
            }
            if file.destination.is_dir() {
                return Err(LumioError::Ingest(format!(
                    "The destination path is a folder, not a file: {}",
                    file.destination.display()
                )));
            }
        }
    }

    let entity_temp = context.temp_root.join(format!(
        "{}-{}",
        context.index,
        &entity.id[..8.min(entity.id.len())]
    ));
    fs::create_dir_all(&entity_temp).map_err(|error| LumioError::io(&entity_temp, error))?;
    let backup_dir = entity_temp.join("backup");
    let mut temp_by_destination: HashMap<PathBuf, PathBuf> = HashMap::new();
    let mut total_files = 0u32;
    let mut total_bytes = 0u64;

    let copy_result = (|| -> Result<()> {
        for (file_index, file) in entity.planned_files.iter().enumerate() {
            if cancel.load(Ordering::SeqCst) {
                return Err(LumioError::Ingest("cancelled".into()));
            }
            let file_name = file
                .destination
                .file_name()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| format!("file-{file_index}"));
            let temp_path = entity_temp.join(format!("{file_index}-{file_name}"));
            match &file.source {
                Some(source) => {
                    copy_and_verify(source, &temp_path)?;
                    run.log.push(log_entry(
                        IngestLogLevel::Info,
                        format!(
                            "Copied and verified {} → {}.",
                            source.display(),
                            file.destination.display()
                        ),
                        Some(file.destination.clone()),
                        Some(entity.id.clone()),
                    ));
                    emit(IngestEvent::FileCopied {
                        entity_id: entity.id.clone(),
                        source: source.clone(),
                        destination: file.destination.clone(),
                        bytes: file.size,
                        index: (file_index + 1) as u32,
                        total: entity.planned_files.len() as u32,
                    });
                }
                None => {
                    create_empty_xmp(&temp_path)?;
                    run.log.push(log_entry(
                        IngestLogLevel::Info,
                        format!("Created XMP sidecar {}.", file.destination.display()),
                        Some(file.destination.clone()),
                        Some(entity.id.clone()),
                    ));
                }
            }
            total_files += 1;
            total_bytes += if file.generated { 0 } else { file.size };
            temp_by_destination.insert(file.destination.clone(), temp_path);
        }

        if !entity.iptc.is_empty() {
            let Some(target) = &entity.metadata_target else {
                return Err(LumioError::Ingest(
                    "IPTC values were requested but no destination metadata file was planned."
                        .into(),
                ));
            };
            let Some(temp_path) = temp_by_destination.get(&target.path) else {
                return Err(LumioError::Ingest(
                    "The destination metadata file was not copied.".into(),
                ));
            };
            let Some(exiftool) = context.plan.exiftool_path.as_deref() else {
                return Err(LumioError::Ingest(
                    "ExifTool is required to write IPTC metadata but was not available.".into(),
                ));
            };
            write_iptc(temp_path, &entity.iptc, exiftool)?;
            run.log.push(log_entry(
                IngestLogLevel::Info,
                format!(
                    "Wrote IPTC title/description/creator/copyright and {} keyword{} to {}.",
                    entity.iptc.keywords.len(),
                    if entity.iptc.keywords.len() == 1 {
                        ""
                    } else {
                        "s"
                    },
                    target.path.display()
                ),
                Some(target.path.clone()),
                Some(entity.id.clone()),
            ));
        }

        if cancel.load(Ordering::SeqCst) {
            return Err(LumioError::Ingest("cancelled".into()));
        }
        Ok(())
    })();

    if let Err(error) = copy_result {
        let _ = fs::remove_dir_all(&entity_temp);
        return Err(error);
    }

    // Commit the whole entity with atomic renames. Existing files are moved
    // aside first so any failure can restore the destination exactly.
    let mut committed: Vec<PathBuf> = Vec::new();
    let mut backups: Vec<(PathBuf, PathBuf)> = Vec::new();

    let commit_result = (|| -> Result<()> {
        fs::create_dir_all(&entity.destination_dir)
            .map_err(|error| LumioError::io(&entity.destination_dir, error))?;
        for (file_index, file) in entity.planned_files.iter().enumerate() {
            let Some(temp_path) = temp_by_destination.get(&file.destination) else {
                return Err(LumioError::Ingest(
                    "A planned copy is missing from the staging folder.".into(),
                ));
            };
            if file.destination.exists() {
                if entity.resolution != IngestConflictResolution::Replace {
                    return Err(LumioError::Ingest(format!(
                        "{} appeared after the dry run. Nothing was overwritten.",
                        file.destination.display()
                    )));
                }
                fs::create_dir_all(&backup_dir)
                    .map_err(|error| LumioError::io(&backup_dir, error))?;
                let backup_path =
                    backup_dir.join(format!("{file_index}-{}", display_name(&file.destination)));
                fs::rename(&file.destination, &backup_path)
                    .map_err(|error| LumioError::io(&file.destination, error))?;
                backups.push((backup_path, file.destination.clone()));
            }
            if let Some(parent) = file.destination.parent() {
                fs::create_dir_all(parent).map_err(|error| LumioError::io(parent, error))?;
            }
            fs::rename(temp_path, &file.destination)
                .map_err(|error| LumioError::io(&file.destination, error))?;
            committed.push(file.destination.clone());
        }
        Ok(())
    })();

    if let Err(error) = commit_result {
        // Roll back the whole entity: remove files we committed, then restore
        // anything that was moved aside for replacement.
        for path in committed.iter().rev() {
            let _ = fs::remove_file(path);
        }
        for (backup, original) in backups.iter().rev() {
            let _ = fs::rename(backup, original);
        }
        let _ = fs::remove_dir_all(&entity_temp);
        return Err(error);
    }

    let _ = fs::remove_dir_all(&entity_temp);
    run.result.status = IngestResultStatus::Copied;
    run.result.files = total_files;
    run.result.bytes = total_bytes;
    run.result.message = Some(format!(
        "Copied {} file{} to {}.",
        total_files,
        if total_files == 1 { "" } else { "s" },
        entity.destination_dir.display()
    ));
    emit(IngestEvent::EntityCompleted {
        entity_id: entity.id.clone(),
        display_name: entity.display_name.clone(),
        destination: entity.destination_dir.clone(),
        files: total_files,
        bytes: total_bytes,
    });
    Ok(run)
}

pub fn execute_plan<F>(plan: &IngestPlan, cancel: &AtomicBool, mut emit: F) -> IngestOutcome
where
    F: FnMut(IngestEvent),
{
    let started = Instant::now();
    let mut logs = plan.log.clone();
    let mut results: Vec<IngestEntityResult> = Vec::new();
    let mut copied_entities = 0u32;
    let mut skipped_entities = 0u32;
    let mut failed_entities = 0u32;
    let mut copied_files = 0u32;
    let mut copied_bytes = 0u64;
    let mut cancelled = false;

    let executable: Vec<&IngestEntity> = plan
        .entities
        .iter()
        .filter(|entity| {
            entity.status != IngestEntityStatus::Error
                && !(entity.resolution == IngestConflictResolution::Skip
                    && entity.conflict.is_some())
        })
        .collect();
    let expected_files: u32 = executable
        .iter()
        .map(|entity| entity.planned_files.len() as u32)
        .sum();
    let expected_bytes: u64 = executable
        .iter()
        .flat_map(|entity| entity.planned_files.iter())
        .map(|file| file.size)
        .sum();

    logs.push(log_entry(
        IngestLogLevel::Info,
        format!(
            "Ingest started for plan {}: {} photo(s), {} file(s).",
            plan.id,
            executable.len(),
            expected_files
        ),
        Some(plan.destination.clone()),
        None,
    ));
    emit(IngestEvent::Started {
        total_entities: executable.len() as u32,
        total_files: expected_files,
        total_bytes: expected_bytes,
    });

    let temp_root = plan.destination.join(".lumio-ingest-tmp").join(&plan.id);
    if let Err(error) = fs::create_dir_all(&temp_root) {
        let message = format!("The ingest staging folder could not be created: {error}");
        logs.push(log_entry(
            IngestLogLevel::Error,
            message.clone(),
            Some(temp_root.clone()),
            None,
        ));
        emit(IngestEvent::Warning {
            entity_id: None,
            message: message.clone(),
        });
        for entity in &plan.entities {
            results.push(IngestEntityResult {
                entity_id: entity.id.clone(),
                display_name: entity.display_name.clone(),
                status: IngestResultStatus::Failed,
                files: 0,
                bytes: 0,
                destination: Some(entity.destination_dir.clone()),
                message: Some(message.clone()),
            });
        }
        return IngestOutcome {
            plan_id: plan.id.clone(),
            destination: plan.destination.clone(),
            copied_entities: 0,
            skipped_entities: 0,
            failed_entities: plan.entities.len() as u32,
            copied_files: 0,
            copied_bytes: 0,
            cancelled: false,
            elapsed_ms: started.elapsed().as_millis() as u64,
            results,
            log: logs,
        };
    }

    let total = plan.entities.len();
    for (index, entity) in plan.entities.iter().enumerate() {
        if cancel.load(Ordering::SeqCst) {
            cancelled = true;
            break;
        }
        if entity.status == IngestEntityStatus::Error {
            failed_entities += 1;
            results.push(IngestEntityResult {
                entity_id: entity.id.clone(),
                display_name: entity.display_name.clone(),
                status: IngestResultStatus::Failed,
                files: 0,
                bytes: 0,
                destination: Some(entity.destination_dir.clone()),
                message: Some("The dry run marked this photo as invalid.".into()),
            });
            continue;
        }
        if entity.resolution == IngestConflictResolution::Skip && entity.conflict.is_some() {
            skipped_entities += 1;
            let reason = entity
                .conflict
                .as_ref()
                .map(|conflict| conflict.message.clone())
                .unwrap_or_else(|| "Destination conflict.".into());
            logs.push(log_entry(
                IngestLogLevel::Warning,
                format!("Skipped {}: {reason}", entity.display_name),
                Some(entity.primary_path.clone()),
                Some(entity.id.clone()),
            ));
            emit(IngestEvent::EntitySkipped {
                entity_id: entity.id.clone(),
                display_name: entity.display_name.clone(),
                reason: reason.clone(),
            });
            results.push(IngestEntityResult {
                entity_id: entity.id.clone(),
                display_name: entity.display_name.clone(),
                status: IngestResultStatus::Skipped,
                files: 0,
                bytes: 0,
                destination: Some(entity.destination_dir.clone()),
                message: Some(reason),
            });
            continue;
        }

        emit(IngestEvent::EntityStarted {
            entity_id: entity.id.clone(),
            display_name: entity.display_name.clone(),
            index: (index + 1) as u32,
            total: total as u32,
        });
        let context = EntityContext {
            plan,
            temp_root: &temp_root,
            index,
        };
        match execute_entity(&context, entity, cancel, &mut emit) {
            Ok(run) => {
                logs.extend(run.log);
                copied_entities += 1;
                copied_files += run.result.files;
                copied_bytes += run.result.bytes;
                logs.push(log_entry(
                    IngestLogLevel::Info,
                    format!(
                        "Copied {} ({} file{}, {} bytes).",
                        entity.display_name,
                        run.result.files,
                        if run.result.files == 1 { "" } else { "s" },
                        run.result.bytes
                    ),
                    Some(entity.destination_dir.clone()),
                    Some(entity.id.clone()),
                ));
                results.push(run.result);
            }
            Err(error) => {
                if error.to_string().contains("cancelled") {
                    cancelled = true;
                    let _ = fs::remove_dir_all(&temp_root);
                    results.push(IngestEntityResult {
                        entity_id: entity.id.clone(),
                        display_name: entity.display_name.clone(),
                        status: IngestResultStatus::Cancelled,
                        files: 0,
                        bytes: 0,
                        destination: Some(entity.destination_dir.clone()),
                        message: Some("Ingest was cancelled.".into()),
                    });
                    break;
                }
                failed_entities += 1;
                let message = error.to_string();
                logs.push(log_entry(
                    IngestLogLevel::Error,
                    format!("Failed {}: {message}", entity.display_name),
                    Some(entity.primary_path.clone()),
                    Some(entity.id.clone()),
                ));
                emit(IngestEvent::EntityFailed {
                    entity_id: entity.id.clone(),
                    display_name: entity.display_name.clone(),
                    message: message.clone(),
                });
                results.push(IngestEntityResult {
                    entity_id: entity.id.clone(),
                    display_name: entity.display_name.clone(),
                    status: IngestResultStatus::Failed,
                    files: 0,
                    bytes: 0,
                    destination: Some(entity.destination_dir.clone()),
                    message: Some(message),
                });
            }
        }
    }

    if cancelled {
        for entity in &plan.entities {
            if results.iter().any(|result| result.entity_id == entity.id) {
                continue;
            }
            results.push(IngestEntityResult {
                entity_id: entity.id.clone(),
                display_name: entity.display_name.clone(),
                status: IngestResultStatus::Cancelled,
                files: 0,
                bytes: 0,
                destination: Some(entity.destination_dir.clone()),
                message: Some("Ingest was cancelled before this photo was processed.".into()),
            });
        }
    }
    let _ = fs::remove_dir_all(&temp_root);
    if let Some(parent) = temp_root.parent() {
        // Only remove the shared staging folder when every plan is gone.
        let _ = fs::remove_dir(parent);
    }

    let summary_message = if cancelled {
        format!(
            "Ingest cancelled: {} photo(s) copied, {} file(s), {} failed.",
            copied_entities, copied_files, failed_entities
        )
    } else {
        format!(
            "Ingest finished: {} photo(s) copied, {} file(s), {} skipped, {} failed.",
            copied_entities, copied_files, skipped_entities, failed_entities
        )
    };
    logs.push(log_entry(
        if failed_entities > 0 {
            IngestLogLevel::Warning
        } else {
            IngestLogLevel::Info
        },
        summary_message,
        Some(plan.destination.clone()),
        None,
    ));

    IngestOutcome {
        plan_id: plan.id.clone(),
        destination: plan.destination.clone(),
        copied_entities,
        skipped_entities,
        failed_entities,
        copied_files,
        copied_bytes,
        cancelled,
        elapsed_ms: started.elapsed().as_millis() as u64,
        results,
        log: logs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::planner::plan_ingest;
    use crate::ingest::types::{IngestFileRole, IngestRequest, IngestRules, XmpNaming};
    use std::sync::atomic::AtomicBool;

    fn rules() -> IngestRules {
        IngestRules {
            folder_template: "{YYYY}/{MM}".into(),
            filename_template: "{default_filename}".into(),
            job_code: String::new(),
            custom_text: String::new(),
            counter_start: 1,
            counter_padding: 4,
            xmp_naming: XmpNaming::FilenameExt,
            rename_postfix: "_#".into(),
            iptc: Default::default(),
        }
    }

    fn plan_for(source: &Path, destination: &Path) -> IngestPlan {
        plan_ingest(&IngestRequest {
            sources: vec![source.to_path_buf()],
            destination: destination.to_path_buf(),
            rules: rules(),
            exiftool_path: None,
        })
        .unwrap()
    }

    #[test]
    fn copies_entities_without_touching_sources() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&destination).unwrap();
        let raw = source.join("photo.CR3");
        let jpeg = source.join("photo.jpg");
        let xmp = source.join("photo.CR3.xmp");
        fs::write(&raw, b"raw-bytes").unwrap();
        fs::write(&jpeg, b"jpeg-bytes").unwrap();
        fs::write(&xmp, b"<xmp/>").unwrap();

        let plan = plan_for(&source, &destination);
        let outcome = execute_plan(&plan, &AtomicBool::new(false), |_| {});
        assert_eq!(outcome.copied_entities, 1);
        assert_eq!(outcome.copied_files, 3);
        assert_eq!(outcome.failed_entities, 0);
        let destination_dir = &plan.entities[0].destination_dir;
        assert_eq!(
            fs::read(destination_dir.join("photo.CR3")).unwrap(),
            b"raw-bytes"
        );
        assert_eq!(
            fs::read(destination_dir.join("photo.jpg")).unwrap(),
            b"jpeg-bytes"
        );
        assert!(destination_dir.join("photo.CR3.xmp").exists());
        assert_eq!(fs::read(&raw).unwrap(), b"raw-bytes");
        assert!(source.exists());
        // Staging is cleaned up.
        assert!(
            !destination
                .join(".lumio-ingest-tmp")
                .join(&plan.id)
                .exists()
        );
    }

    #[test]
    fn skip_resolution_keeps_existing_file() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(source.join("photo.CR3"), b"new").unwrap();

        let plan = plan_for(&source, &destination);
        let target = plan.entities[0].planned_files[0].destination.clone();
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, b"old").unwrap();
        let plan = plan_for(&source, &destination);
        assert_eq!(plan.entities[0].status, IngestEntityStatus::Conflict);

        let outcome = execute_plan(&plan, &AtomicBool::new(false), |_| {});
        assert_eq!(outcome.skipped_entities, 1);
        assert_eq!(outcome.copied_entities, 0);
        assert_eq!(fs::read(&target).unwrap(), b"old");
    }

    #[test]
    fn replace_requires_authorization_and_restores_on_success() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(source.join("photo.CR3"), b"new").unwrap();

        let plan = plan_for(&source, &destination);
        let target = plan.entities[0].planned_files[0].destination.clone();
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, b"old").unwrap();
        let mut plan = plan_for(&source, &destination);
        let id = plan.entities[0].id.clone();
        crate::ingest::planner::set_entity_resolution(
            &mut plan,
            &id,
            IngestConflictResolution::Replace,
        )
        .unwrap();

        let outcome = execute_plan(&plan, &AtomicBool::new(false), |_| {});
        assert_eq!(outcome.failed_entities, 0);
        assert_eq!(fs::read(&target).unwrap(), b"new");
    }

    #[test]
    fn cancellation_stops_before_commit() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(source.join("a.CR3"), b"a").unwrap();
        fs::write(source.join("b.CR3"), b"b").unwrap();

        let plan = plan_for(&source, &destination);
        let cancel = AtomicBool::new(false);
        let outcome = execute_plan(&plan, &cancel, |_| {});
        assert_eq!(outcome.copied_entities, 2);

        let cancel = AtomicBool::new(true);
        let plan2 = plan_for(&source, &destination);
        let outcome = execute_plan(&plan2, &cancel, |_| {});
        assert!(outcome.cancelled);
        assert_eq!(outcome.copied_entities, 0);
    }

    #[test]
    fn writes_iptc_to_a_generated_sidecar_during_ingest() {
        if crate::ingest::metadata::discover_exiftool(None).is_none() {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(source.join("photo.CR3"), b"raw-bytes").unwrap();

        let mut rules = rules();
        rules.folder_template = String::new();
        rules.iptc.title = "Morning light".into();
        rules.iptc.keywords = vec!["travel".into(), "beach".into()];
        let plan = plan_ingest(&IngestRequest {
            sources: vec![source.clone()],
            destination: destination.clone(),
            rules,
            exiftool_path: None,
        })
        .unwrap();
        assert_eq!(plan.entities[0].planned_files.len(), 2);
        assert!(
            plan.entities[0]
                .planned_files
                .iter()
                .any(|file| file.generated && file.role == IngestFileRole::Xmp)
        );

        let outcome = execute_plan(&plan, &AtomicBool::new(false), |_| {});
        assert_eq!(outcome.failed_entities, 0);
        let sidecar = destination.join("photo.CR3.xmp");
        assert!(sidecar.exists());
        let content = fs::read_to_string(&sidecar).unwrap();
        assert!(content.contains("Morning light"));
        assert!(content.contains("travel"));
        assert!(content.contains("beach"));
        // The original and its folder are untouched.
        assert_eq!(fs::read(source.join("photo.CR3")).unwrap(), b"raw-bytes");
    }

    #[test]
    fn duplicate_plan_auto_renames_and_copies_both() {
        let temp = tempfile::tempdir().unwrap();
        let a = temp.path().join("a");
        let b = temp.path().join("b");
        let destination = temp.path().join("destination");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(a.join("same.CR3"), b"one").unwrap();
        fs::write(b.join("same.CR3"), b"two").unwrap();

        let plan = plan_ingest(&IngestRequest {
            sources: vec![a, b],
            destination: destination.clone(),
            rules: rules(),
            exiftool_path: None,
        })
        .unwrap();
        let outcome = execute_plan(&plan, &AtomicBool::new(false), |_| {});
        assert_eq!(outcome.copied_entities, 2);
        let stems: Vec<String> = plan
            .entities
            .iter()
            .map(|entity| entity.destination_stem.clone())
            .collect();
        assert!(stems.contains(&"same".to_string()));
        assert!(stems.contains(&"same_2".to_string()));
        assert!(destination.join(&plan.entities[0].destination_dir).exists());
    }
}
