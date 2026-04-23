const BANNER: &str = r#"
   __  __           _
  |  \/  |         | |
  | \  / | ___ _ __| | __
  | |\/| |/ _ \ '__| |/ /
  | |  | |  __/ |  |   <
  |_|  |_|\___|_|  |_|\_\
"#;

pub fn log_startup(base_url: &str) {
    tracing::info!("{}", BANNER);
    tracing::info!("Project : merk");
    tracing::info!("Author  : Usairim Isani");
    tracing::info!("API     : {}", base_url);
    tracing::info!("Docs    : {}/docs/scalar", base_url);
    tracing::info!("GraphQL : {}/graphql", base_url);
}
