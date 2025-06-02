//! This module manages the state of started agents.
//! It keeps track of the agents that have been started, their workers, and their addresses.
//! It keeps two copies of the state to reduce contention.
//! The list_agents method is non-authoritative and returns the a state that is at most 100ms old.

use ockam::Address;
use ockam::compat::asynchronous::{Mutex as AsyncMutex, RwLock as AsyncRwLock};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone, Default)]
struct StartedAgentsAuthoritative {
    agents: HashMap<String, StartedAgent>,
    last_copy: Option<Instant>,
}

#[derive(Clone, Default)]
struct StartedAgentsCopy {
    agents: HashMap<String, StartedAgent>,
    last_copy: Option<Instant>,
}

#[derive(Clone, Default)]
pub(crate) struct StartedAgents {
    inner: Arc<AsyncRwLock<StartedAgentsAuthoritative>>,
    copy: Arc<AsyncMutex<StartedAgentsCopy>>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct StartedAgent {
    name: String,
    is_remote: bool,
    description: Option<String>,
    workers: HashMap<String, String>,
}

impl StartedAgent {
    pub(crate) fn output(&self) -> AgentOutput {
        AgentOutput {
            name: self.name.clone(),
            is_remote: self.is_remote,
            description: self.description.clone(),
            workers: self.workers.keys().cloned().collect(),
        }
    }
}

const UPDATE_INTERVAL: u128 = 100;

impl StartedAgents {
    pub(crate) async fn started(&self, name: String, is_remote: bool, description: Option<String>) {
        let agent = StartedAgent {
            name: name.clone(),
            is_remote,
            description,
            workers: HashMap::new(),
        };

        let now;
        let new_value = {
            let mut guard = self.inner.write().await;
            now = Instant::now();
            guard.agents.insert(name, agent);

            let should_update = guard
                .last_copy
                .map(|last| now.duration_since(last).as_millis() >= UPDATE_INTERVAL)
                .unwrap_or(true);
            if should_update {
                guard.last_copy = Some(now);
                Some(guard.agents.clone())
            } else {
                None
            }
        };

        if let Some(new_value) = new_value {
            let mut guard = self.copy.lock().await;
            guard.agents = new_value;
            guard.last_copy = Some(now);
        }
    }

    pub(crate) async fn add_worker(&self, agent_name: &str, key: String, address: &Address) {
        let address = address.to_string();

        let now;
        let new_value = {
            let mut guard = self.inner.write().await;
            now = Instant::now();
            if let Some(agent) = guard.agents.get_mut(agent_name) {
                agent.workers.insert(key, address);
            }

            let should_update = guard
                .last_copy
                .map(|last| now.duration_since(last).as_millis() >= UPDATE_INTERVAL)
                .unwrap_or(true);

            if should_update {
                guard.last_copy = Some(now);
                Some(guard.agents.clone())
            } else {
                None
            }
        };

        if let Some(new_value) = new_value {
            let mut guard = self.copy.lock().await;
            guard.agents = new_value;
            guard.last_copy = Some(now);
        }
    }

    pub(crate) async fn get_worker(&self, agent_name: &str, key: &str) -> Option<Address> {
        let guard = self.inner.read().await;
        guard
            .agents
            .get(agent_name)
            .and_then(|agent| agent.workers.get(key))
            .map(|a| a.into())
    }

    pub(crate) async fn stopped(&self, name: &str) {
        let now;

        let new_value = {
            let mut guard = self.inner.write().await;
            now = Instant::now();
            guard.agents.remove(name);

            let must_update = guard
                .last_copy
                .map(|last| now.duration_since(last).as_millis() >= UPDATE_INTERVAL)
                .unwrap_or(true);

            if must_update {
                guard.last_copy = Some(now);
                Some(guard.agents.clone())
            } else {
                None
            }
        };

        if let Some(new_value) = new_value {
            let mut guard = self.copy.lock().await;
            guard.agents = new_value;
            guard.last_copy = Some(now);
        }
    }

    pub(crate) async fn list_agents(&self) -> Vec<AgentOutput> {
        {
            let guard = self.copy.lock().await;

            let now = Instant::now();
            let must_update = if let Some(last_copy) = guard.last_copy {
                now.duration_since(last_copy).as_millis() >= UPDATE_INTERVAL
            } else {
                true
            };

            if !must_update {
                return guard.agents.values().map(|a| a.output()).collect();
            }
        }

        // release the copy lock, as locking with a reverse order causes deadlocks
        let mut inner_guard = self.inner.write().await;
        let now = Instant::now();

        inner_guard.last_copy = Some(now);

        let mut copy_guard = self.copy.lock().await;
        copy_guard.agents = inner_guard.agents.clone();
        copy_guard.last_copy = Some(now);

        copy_guard.agents.values().map(|a| a.output()).collect()
    }

    pub(crate) async fn list_agents_as_json(&self) -> Value {
        serde_json::to_value(self.list_agents().await).unwrap_or_else(|_| Value::Array(vec![]))
    }
}

/// We use this struct to only expose data that is needed for the API.
#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct AgentOutput {
    pub(crate) name: String,
    pub(crate) is_remote: bool,
    pub(crate) description: Option<String>,
    pub(crate) workers: Vec<String>,
}
