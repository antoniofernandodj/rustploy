use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod source;
mod transform;
mod sink;
mod warnings;

#[derive(Parser)]
#[command(name = "rustploy-import")]
#[command(about = "Migrate data from other platforms to Rustploy", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Import data from a Dokploy instance
    Dokploy {
        #[arg(long, default_value = "postgresql://dokploy:dokploy@localhost:5432/dokploy")]
        pg_url: String,

        #[arg(long)]
        auto_detect_docker: bool,

        #[arg(long)]
        gitea_url: Option<String>,

        #[arg(long)]
        dry_run: bool,

        #[arg(long)]
        output_sql: Option<String>,

        #[arg(long, short)]
        yes: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Dokploy {
            pg_url,
            auto_detect_docker,
            gitea_url,
            dry_run,
            output_sql,
            yes,
        } => {
            let final_pg_url = pg_url;
            if auto_detect_docker {
                // TODO: Logic to detect dokploy-postgres container and get its IP/Port
                // For now, assume localhost if running on the host
            }

            println!("🚀 Iniciando migração do Dokploy...");
            
            // 1. Source
            let source = source::dokploy::DokploySource::new(&final_pg_url).await?;
            let data = source.fetch_all().await?;
            
            println!("✅ Dados extraídos do Dokploy:");
            println!("   - Projetos: {}", data.projects.len());
            println!("   - Aplicações: {}", data.applications.len());
            println!("   - Compose: {}", data.composes.len());

            // 2. Transform
            let (transformed, report) = transform::dokploy::transform(data, gitea_url.as_deref());
            
            // 3. Warnings
            report.print();
            
            if report.has_blocking() {
                println!("\n❌ Migração interrompida devido a erros bloqueantes.");
                std::process::exit(1);
            }

            if dry_run {
                println!("\n✨ Dry-run concluído. Nenhuma alteração foi feita.");
                return Ok(());
            }

            if !yes {
                // TODO: Interactive confirmation if not --yes
                println!("\n⚠️  Aviso: Use --yes para confirmar a importação.");
                return Ok(());
            }

            // 4. Sink
            if let Some(path) = output_sql {
                sink::write_sql_file(&path, &transformed).await?;
                println!("\n💾 SQL de migração gerado em: {}", path);
            } else {
                sink::write_to_db(&transformed).await?;
                println!("\n🎉 Migração concluída com sucesso!");
            }
        }
    }

    Ok(())
}
