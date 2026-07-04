    /// Ensure a session exists. If it does not, create a minimal default one.
    /// This prevents “Session not found” errors on first-use or when the client
    /// sends a slash command before the normal session/new handshake.
    async fn ensure_session(&self, session_id: &str) -> SessionData {
        {
            let sessions = self.sessions.read().await;
            if let Some(s) = sessions.get(session_id) {
                return s.clone();
            }
        }

        // Create a minimal default session
        let mut session_data = SessionData {
            cwd: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string()),
            messages: Vec::new(),
            config: SessionConfig::default(),
            created_at: std::time::Instant::now(),
            last_activity: std::time::Instant::now(),
            always_allow: std::collections::HashSet::new(),
            client_commands: Vec::new(),
            bayes_engine: crate::bayes::BayesianEngine::new_with_config(&self.config.bayesian),
            dna: crate::session::dna::SessionDna::default(),
            current_goal: None,
        };

        // Inject DNA into the (empty) system prompt so the session is usable
        let dna = crate::session::dna::SessionDna::load();
        let mut prompt = String::new();
        dna.inject_into_prompt(&mut prompt);
        prompt.push_str(&format!("\n\n**Current DNA Mode:** {}", dna.get_mode()));
        if !prompt.trim().is_empty() {
            session_data.messages.push(serde_json::json!({
                "role": "system",
                "content": prompt.trim().to_string(),
            }));
        }
        session_data.dna = dna;

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.to_string(), session_data.clone());
        info!("Auto-created minimal session for missing ID: {}", session_id);
        session_data
    }