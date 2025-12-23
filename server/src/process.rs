use std::process::Stdio;
use tokio::process::{Child, Command};
use anyhow::{Result, Context};

pub struct ProcessManager {
    child: Option<Child>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self { child: None }
    }

    pub fn start(&mut self, _model_path: &str, args: &[String]) -> Result<()> {
        // In a real scenario, we might use model_path as the -m argument if not provided in args,
        // or ensure it's passed correctly. For now, we assume args contains everything needed or we append it.
        // However, looking at the config, 'path' is separate. Let's construct the command properly.
        
        tracing::info!("Starting llama-server with args: {:?}", args);

        let mut cmd = Command::new("llama-server");
        
        // If we wanted to enforce the model path here:
        // cmd.arg("-m").arg(model_path);
        // But the user might have put "-m" in args. 
        // For simplicity in this iteration, let's assume 'args' from config is comprehensive 
        // OR we prepend the model path if it's not in args.
        // Let's stick to the plan: pass args directly. 
        // NOTE: The implementation plan implies using model_path designated in config.
        
        // Let's actually append -m model_path if it's not present, or assume the caller handles it.
        // For safer implementation, let's look at how we'll call it.
        // We will pass `config.path` and `config.args`. 
        // `llama-server -m <path> <args>` is the standard way.
        
        cmd.arg("-m").arg(_model_path);
        cmd.args(args);

        // Standard ports configuration could also be enforced here if needed, 
        // but let's assume args or default logic.
        // Adding --port 8080 explicitly if not in args could be good, but let's trust the config/defaults.
        // We'll add a default port if not specified? 
        // Actually, let's stick to hardcoded 8080 for the server code's expectation for now, 
        // or just let it be.
        
        cmd.arg("--port").arg("8080");

        cmd.stdout(Stdio::null()); // or piped for logging
        cmd.stderr(Stdio::null());

        let child = cmd.spawn().context("Failed to spawn llama-server")?;
        self.child = Some(child);
        
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            tracing::info!("Stopping llama-server process...");
            child.kill().await.context("Failed to kill llama-server process")?;
            child.wait().await.context("Failed to wait for llama-server process termination")?;
        }
        Ok(())
    }

    pub async fn restart(&mut self, model_path: &str, args: &[String]) -> Result<()> {
        self.stop().await?;
        self.start(model_path, args)?;
        Ok(())
    }
}
