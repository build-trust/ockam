use crate::address::process_nodes_multiaddr;
use crate::{CliState, DefaultAddress, ParseError};
use miette::IntoDiagnostic;
use ockam_multiaddr::{proto, MultiAddr, Protocol};
use std::str::FromStr;

pub fn tcp_inlet_default_to_address() -> String {
    "/project/<default_project_name>/service/forward_to_<default_relay_name>/secure/api/service/<default_service_name>".to_string()
}

pub async fn parse_to_address(
    state: &CliState,
    to: impl Into<String>,
    via: Option<&String>,
) -> miette::Result<String> {
    let mut to = to.into();
    let to_is_default = to == tcp_inlet_default_to_address();
    let mut service_name = DefaultAddress::OUTLET_SERVICE.to_string();
    let relay_name = via.cloned().unwrap_or("default".to_string());

    match MultiAddr::from_str(&to) {
        // "to" is a valid multiaddr
        Ok(route) => {
            // check whether it's a full route or a single service
            let protos = route.iter().collect::<Vec<_>>();
            if let Some(proto) = protos.first() {
                // "to" refers to the service name
                if proto.code() == proto::Service::CODE && protos.len() == 1 {
                    service_name = proto
                        .cast::<proto::Service>()
                        .ok_or_else(|| ParseError::validation("to", via, None))?
                        .to_string();

                    // if "via" is passed, then we reset "to" to the default value
                    if via.is_some() {
                        to = tcp_inlet_default_to_address();
                    }
                }
                // "to" is a full route
                else {
                    // "via" can't be passed if the user provides a value for "to"
                    if !to_is_default && via.is_some() {
                        return Err(ParseError::validation(
                            "to",
                            via,
                            Some("'via' can't be used if 'to' is a full route"),
                        ))?;
                    }
                }
            }
        }
        // If it's not
        Err(_) => {
            // "to" refers to the service name
            service_name = to.to_string();
            // and we set "to" to the default route, so we can do the replacements later
            to = tcp_inlet_default_to_address();
        }
    }

    // Replace the placeholders
    if to.contains("<default_project_name>") {
        let project_name = state
            .projects()
            .get_default_project()
            .await
            .map(|p| p.name().to_string())
            .ok()
            .ok_or(ParseError::validation("to", via, Some("No projects found")))?;
        to = to.replace("<default_project_name>", &project_name);
    }
    to = to.replace("<default_relay_name>", &relay_name);
    to = to.replace("<default_service_name>", &service_name);

    // Parse "to" as a multiaddr again with all the values in place
    let to = MultiAddr::from_str(&to).into_diagnostic()?;
    Ok(process_nodes_multiaddr(&to, state).await?.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::lookup::InternetAddress;
    use crate::orchestrator::project::models::ProjectModel;
    use crate::orchestrator::project::Project;
    use ockam::identity::Identifier;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_parse_to_address() {
        let state = CliState::test().await.unwrap();
        setup_state(&state).await;

        let node_name = "n1";
        let node_port = state
            .get_node(node_name)
            .await
            .unwrap()
            .tcp_listener_port()
            .unwrap();

        // Invalid "to" values throw an error
        let cases = ["/alice/service", "alice/relay"];
        for to in cases {
            parse_to_address(&state, to, None)
                .await
                .expect_err("Invalid multiaddr");
        }

        // "to" with default value expands the placeholders
        let res = parse_to_address(&state, tcp_inlet_default_to_address(), None)
            .await
            .unwrap();
        assert_eq!(
            res,
            "/project/p1/service/forward_to_default/secure/api/service/outlet".to_string()
        );

        // "to" argument accepts a full route
        let cases = [
            ("/project/p2/service/forward_to_n1/secure/api/service/myoutlet", None),
            ("/worker/603b62d245c9119d584ba3d874eb8108/service/forward_to_n3/service/hop/service/outlet", None),
            (&format!("/node/{node_name}/service/myoutlet"), Some(format!("/ip4/127.0.0.1/tcp/{node_port}/service/myoutlet"))),
        ];
        for (to, expected) in cases {
            let res = parse_to_address(&state, to, None).await.unwrap();
            let expected = expected.unwrap_or(to.to_string());
            assert_eq!(res, expected);
        }

        // "to" argument accepts the name of the service
        let res = parse_to_address(&state, "myoutlet", None).await.unwrap();
        assert_eq!(
            res,
            "/project/p1/service/forward_to_default/secure/api/service/myoutlet".to_string()
        );

        // "via" argument is used to replace the relay name, and "to" is a service name
        let cases = [
            (
                tcp_inlet_default_to_address(),
                "myrelay",
                "/project/p1/service/forward_to_myrelay/secure/api/service/outlet",
            ),
            (
                "myoutlet".to_string(),
                "myrelay",
                "/project/p1/service/forward_to_myrelay/secure/api/service/myoutlet",
            ),
            (
                "/service/myoutlet".to_string(),
                "myrelay",
                "/project/p1/service/forward_to_myrelay/secure/api/service/myoutlet",
            ),
        ];
        for (to, via, expected) in cases {
            let res = parse_to_address(&state, &to, Some(&via.to_string()))
                .await
                .unwrap();
            assert_eq!(res, expected.to_string());
        }

        // if "to" is passed as a full route and also "via" is passed, return an error
        let to = "/project/p1/service/forward_to_n1/secure/api/service/outlet";
        parse_to_address(&state, to, Some(&"myrelay".to_string()))
            .await
            .expect_err("'via' can't be passed if 'to' is a full route");
    }

    // UTILS

    async fn setup_state(state: &CliState) {
        let project = ProjectModel {
            identity: Some(
                Identifier::from_str(
                    "Ie92f183eb4c324804ef4d62962dea94cf095a265a1b2c3d4e5f6a6b5c4d3e2f1",
                )
                .unwrap(),
            ),
            name: "p1".to_string(),
            ..Default::default()
        };
        let project = Project::import(project).await.unwrap();
        state.projects().store_project(project).await.unwrap();

        state.create_node("n1").await.unwrap();
        state
            .set_tcp_listener_address("n1", &InternetAddress::new("127.0.0.1:1234").unwrap())
            .await
            .unwrap();
    }
}
