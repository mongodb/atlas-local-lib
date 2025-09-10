use bollard::query_parameters::ListContainersOptionsBuilder;
use maplit::hashmap;

use crate::{
    client::Client,
    docker::{DockerInspectContainer, DockerListContainers},
    models::{Deployment, LOCAL_DEPLOYMENT_LABEL_KEY, LOCAL_DEPLOYMENT_LABEL_VALUE},
};

use super::GetDeploymentError;

impl<D: DockerListContainers + DockerInspectContainer> Client<D> {
    /// Lists all local Atlas deployments.
    pub async fn list_deployments(&self) -> Result<Vec<Deployment>, GetDeploymentError> {
        // Build the list containers options which will filter for containers with the local deployment label
        let list_container_options = ListContainersOptionsBuilder::default()
            .all(true)
            .filters(&hashmap! {
                "label" => vec![format!("{}={}", LOCAL_DEPLOYMENT_LABEL_KEY, LOCAL_DEPLOYMENT_LABEL_VALUE)],
            })
            .build();

        // Get all the containers using the list containers options
        let container_summaries = self
            .docker
            .list_containers(Some(list_container_options))
            .await?;

        // Create the output vector used to return the deployments
        let mut deployments = Vec::with_capacity(container_summaries.len());

        // Iterate over the container summaries and get the deployment details for each container
        for container_summary in container_summaries {
            // Get the container ID from the container summary
            // This should always be present, but it's cleaner to not use unwrap and skip if it's not present
            if let Some(container_id) = container_summary.id {
                // Get the deployment details for the container
                let deployment = self.get_deployment(container_id.as_ref()).await?;
                deployments.push(deployment);
            }
        }

        Ok(deployments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MongodbType, State};
    use bollard::{
        errors::Error as BollardError,
        query_parameters::{InspectContainerOptions, ListContainersOptions},
        secret::{
            ContainerConfig, ContainerInspectResponse, ContainerState, ContainerStateStatusEnum,
            ContainerSummary,
        },
    };
    use mockall::mock;
    use std::collections::HashMap;

    mock! {
        Docker {}

        impl DockerListContainers for Docker {
            async fn list_containers(
                &self,
                options: Option<ListContainersOptions>,
            ) -> Result<Vec<ContainerSummary>, BollardError>;
        }

        impl DockerInspectContainer for Docker {
            async fn inspect_container(
                &self,
                container_id: &str,
                options: Option<InspectContainerOptions>,
            ) -> Result<ContainerInspectResponse, BollardError>;
        }
    }

    fn create_container_summary(id: &str, name: &str) -> ContainerSummary {
        ContainerSummary {
            id: Some(id.to_string()),
            names: Some(vec![format!("/{}", name)]),
            ..Default::default()
        }
    }

    fn create_container_inspect_response(id: &str, name: &str) -> ContainerInspectResponse {
        let mut labels = HashMap::new();
        labels.insert("mongodb-atlas-local".to_string(), "container".to_string());
        labels.insert("version".to_string(), "8.0.0".to_string());
        labels.insert("mongodb-type".to_string(), "community".to_string());

        let env_vars = vec!["TOOL=ATLASCLI".to_string()];

        ContainerInspectResponse {
            id: Some(id.to_string()),
            name: Some(format!("/{}", name)),
            config: Some(ContainerConfig {
                labels: Some(labels),
                env: Some(env_vars),
                ..Default::default()
            }),
            state: Some(ContainerState {
                status: Some(ContainerStateStatusEnum::RUNNING),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_list_deployments() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        let container_summaries = vec![
            create_container_summary("container1", "deployment1"),
            create_container_summary("container2", "deployment2"),
        ];

        let container_inspect_response1 =
            create_container_inspect_response("container1", "deployment1");
        let container_inspect_response2 =
            create_container_inspect_response("container2", "deployment2");

        // Set up expectations
        mock_docker
            .expect_list_containers()
            .times(1)
            .returning(move |_| Ok(container_summaries.clone()));

        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("container1"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(move |_, _| Ok(container_inspect_response1.clone()));

        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("container2"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(move |_, _| Ok(container_inspect_response2.clone()));

        let client = Client::new(mock_docker);

        // Act
        let result = client.list_deployments().await;

        // Assert
        assert!(result.is_ok());
        let deployments = result.unwrap();
        assert_eq!(deployments.len(), 2);

        assert_eq!(deployments[0].container_id, "container1");
        assert_eq!(deployments[0].name, Some("deployment1".to_string()));
        assert_eq!(deployments[0].state, State::Running);
        assert_eq!(deployments[0].mongodb_type, MongodbType::Community);

        assert_eq!(deployments[1].container_id, "container2");
        assert_eq!(deployments[1].name, Some("deployment2".to_string()));
        assert_eq!(deployments[1].state, State::Running);
        assert_eq!(deployments[1].mongodb_type, MongodbType::Community);
    }

    #[tokio::test]
    async fn test_list_deployments_empty() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        // Set up expectations
        mock_docker
            .expect_list_containers()
            .times(1)
            .returning(|_| Ok(vec![]));

        let client = Client::new(mock_docker);

        // Act
        let result = client.list_deployments().await;

        // Assert
        assert!(result.is_ok());
        let deployments = result.unwrap();
        assert_eq!(deployments.len(), 0);
    }

    #[tokio::test]
    async fn test_list_deployments_list_containers_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        // Set up expectations
        mock_docker
            .expect_list_containers()
            .times(1)
            .returning(|_| {
                Err(BollardError::DockerResponseServerError {
                    status_code: 500,
                    message: "Internal Server Error".to_string(),
                })
            });

        let client = Client::new(mock_docker);

        // Act
        let result = client.list_deployments().await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GetDeploymentError::ContainerInspect(_)
        ));
    }

    #[tokio::test]
    async fn test_list_deployments_inspect_container_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        let container_summaries = vec![create_container_summary("container1", "deployment1")];

        // Set up expectations
        mock_docker
            .expect_list_containers()
            .times(1)
            .returning(move |_| Ok(container_summaries.clone()));

        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("container1"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(|_, _| {
                Err(BollardError::DockerResponseServerError {
                    status_code: 404,
                    message: "No such container".to_string(),
                })
            });

        let client = Client::new(mock_docker);

        // Act
        let result = client.list_deployments().await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GetDeploymentError::ContainerInspect(_)
        ));
    }

    #[tokio::test]
    async fn test_list_deployments_skip_containers_without_id() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        let container_summaries = vec![
            ContainerSummary {
                id: None, // Container without ID should be skipped
                ..Default::default()
            },
            create_container_summary("container2", "deployment2"),
        ];

        let container_inspect_response2 =
            create_container_inspect_response("container2", "deployment2");

        // Set up expectations
        mock_docker
            .expect_list_containers()
            .times(1)
            .returning(move |_| Ok(container_summaries.clone()));

        mock_docker
            .expect_inspect_container()
            .with(
                mockall::predicate::eq("container2"),
                mockall::predicate::eq(None::<InspectContainerOptions>),
            )
            .times(1)
            .returning(move |_, _| Ok(container_inspect_response2.clone()));

        let client = Client::new(mock_docker);

        // Act
        let result = client.list_deployments().await;

        // Assert
        assert!(result.is_ok());
        let deployments = result.unwrap();
        assert_eq!(deployments.len(), 1); // Only one deployment should be returned
        assert_eq!(deployments[0].container_id, "container2");
        assert_eq!(deployments[0].name, Some("deployment2".to_string()));
    }
}
