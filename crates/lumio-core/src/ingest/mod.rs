pub mod association;
pub mod executor;
pub mod iptc;
pub mod metadata;
pub mod planner;
pub mod template;
pub mod types;

pub use executor::execute_plan;
pub use metadata::{detect_tools, discover_exiftool};
pub use planner::{plan_ingest, rebuild_plan_paths, set_entity_resolution};
pub use template::{
    TemplateContext, TemplateKind, render, render_folder_segments, validate_rendered_filename,
    validate_required_inputs, validate_template,
};
pub use types::*;
