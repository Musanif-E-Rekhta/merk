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
    tracing::info!("Docs    : {}/docs/scalar", base_url);
    tracing::info!("API     : {}/api/v1", base_url);
    tracing::info!("GraphQL : {}/api/graphql", base_url);
}
