use crate::{
    Client,
    client::get_deployment::GetDeploymentError,
    docker::{DockerInspectContainer, RunCommandInContainer, RunCommandInContainerError},
    models::Deployment,
};

#[derive(Debug, thiserror::Error)]
pub enum GetDeploymentIdError {
    #[error("Failed to get deployment: {0}")]
    GetDeployment(#[from] GetDeploymentError),
    #[error("Failed to get MongoDB username: {0}")]
    GetMongodbUsername(RunCommandInContainerError),
    #[error("Failed to get MongoDB password: {0}")]
    GetMongodbPassword(RunCommandInContainerError),
    #[error("Failed to run mongosh command: {0}")]
    RunMongoshCommand(RunCommandInContainerError),
    #[error("Deployment ID is empty")]
    DeploymentIdEmpty,
}

impl<D: DockerInspectContainer + RunCommandInContainer> Client<D> {
    /// Gets the Atlas deployment ID for a local Atlas deployment.
    pub async fn get_deployment_id(
        &self,
        cluster_id_or_name: &str,
    ) -> Result<String, GetDeploymentIdError> {
        let deployment = self.get_deployment(cluster_id_or_name).await?;

        // Try to get the MongoDB root username
        let mongodb_root_username = self
            .get_mongodb_secret(
                &deployment,
                |d| d.mongodb_initdb_root_username.as_deref(),
                |d| d.mongodb_initdb_root_username_file.as_deref(),
            )
            .await
            .map_err(GetDeploymentIdError::GetMongodbUsername)?;

        // Try to get the MongoDB root password
        let mongodb_root_password = self
            .get_mongodb_secret(
                &deployment,
                |d| d.mongodb_initdb_root_password.as_deref(),
                |d| d.mongodb_initdb_root_password_file.as_deref(),
            )
            .await
            .map_err(GetDeploymentIdError::GetMongodbPassword)?;

        // Build the mongosh command
        let mut mongosh_command = vec![
            "mongosh".to_string(),
            "mongodb://127.0.0.1:27017/?directConnection=true".to_string(),
        ];
        if let Some(username) = mongodb_root_username {
            mongosh_command.push(format!("--username={}", username));
        }
        if let Some(password) = mongodb_root_password {
            mongosh_command.push(format!("--password={}", password));
        }

        mongosh_command.push("--eval".to_string());
        mongosh_command.push("db.getSiblingDB('admin').atlascli.findOne()?.uuid".to_string());
        mongosh_command.push("--quiet".to_string());

        // Run the mongosh command
        let command_output = self
            .docker
            .run_command_in_container(&deployment.container_id, mongosh_command)
            .await
            .map_err(GetDeploymentIdError::RunMongoshCommand)?;

        match command_output.stdout.into_iter().next() {
            Some(line) if line.is_empty() => Err(GetDeploymentIdError::DeploymentIdEmpty),
            Some(line) => Ok(line),
            None => Err(GetDeploymentIdError::DeploymentIdEmpty),
        }
    }

    async fn get_mongodb_secret(
        &self,
        deployment: &Deployment,
        value: impl FnOnce(&Deployment) -> Option<&str>,
        file: impl FnOnce(&Deployment) -> Option<&str>,
    ) -> Result<Option<String>, RunCommandInContainerError> {
        // Try to get the value from the environment variables first
        if let Some(env_value) = value(deployment) {
            return Ok(Some(env_value.to_string()));
        }

        // If the value is not found in the environment variables, try to get it from the file
        if let Some(file_value) = file(deployment) {
            let command_output = self
                .docker
                .run_command_in_container(
                    &deployment.container_id,
                    vec!["cat".to_string(), file_value.to_string()],
                )
                .await?;

            if let Some(line) = command_output.stdout.into_iter().next() {
                return Ok(Some(line));
            }
        }

        // If the value is not found in the environment variables or the file, return None
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{client::get_deployment::GetDeploymentError, docker::CommandOutput};
    use bollard::{
        errors::Error as BollardError,
        query_parameters::InspectContainerOptions,
        secret::{
            ContainerConfig, ContainerInspectResponse, ContainerState, ContainerStateStatusEnum,
        },
    };
    use maplit::hashmap;
    use mockall::{mock, predicate::eq};

    mock! {
        Docker {}

        impl DockerInspectContainer for Docker {
            async fn inspect_container(
                &self,
                container_id: &str,
                options: Option<InspectContainerOptions>,
            ) -> Result<ContainerInspectResponse, BollardError>;
        }

        impl RunCommandInContainer for Docker {
            async fn run_command_in_container(
                &self,
                container_id: &str,
                command: Vec<String>,
            ) -> Result<CommandOutput, RunCommandInContainerError>;
        }
    }

    // Helper function to create a test container inspect response
    fn create_test_container_inspect_response() -> ContainerInspectResponse {
        ContainerInspectResponse {
            id: Some("test_container_id".to_string()),
            name: Some("/test-deployment".to_string()),
            config: Some(ContainerConfig {
                labels: Some(hashmap! {
                    "mongodb-atlas-local".to_string() => "container".to_string(),
                    "version".to_string() => "8.0.0".to_string(),
                    "mongodb-type".to_string() => "community".to_string(),
                }),
                env: Some(vec!["TOOL=ATLASCLI".to_string()]),
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
    async fn test_get_deployment_id_happy_path_no_auth() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let container_inspect_response = create_test_container_inspect_response();

        // Mock get_deployment call
        mock_docker
            .expect_inspect_container()
            .with(eq("test-deployment"), eq(None::<InspectContainerOptions>))
            .times(1)
            .returning(move |_, _| Ok(container_inspect_response.clone()));

        // Mock mongosh command execution
        mock_docker
            .expect_run_command_in_container()
            .with(
                eq("test_container_id"),
                eq(vec![
                    "mongosh".to_string(),
                    "mongodb://127.0.0.1:27017/?directConnection=true".to_string(),
                    "--eval".to_string(),
                    "db.getSiblingDB('admin').atlascli.findOne()?.uuid".to_string(),
                    "--quiet".to_string(),
                ]),
            )
            .times(1)
            .returning(|_, _| {
                Ok(CommandOutput {
                    stdout: vec!["deployment-uuid-123".to_string()],
                    stderr: vec![],
                })
            });

        let client = Client::new(mock_docker);

        // Act
        let result = client.get_deployment_id("test-deployment").await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "deployment-uuid-123");
    }

    #[tokio::test]
    async fn test_get_deployment_id_happy_path_env_auth() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let mut container_inspect_response = create_test_container_inspect_response();

        // Add username and password environment variables
        if let Some(config) = container_inspect_response.config.as_mut()
            && let Some(env) = config.env.as_mut()
        {
            env.push("MONGODB_INITDB_ROOT_USERNAME=testuser".to_string());
            env.push("MONGODB_INITDB_ROOT_PASSWORD=testpass".to_string());
        }

        // Mock get_deployment call
        mock_docker
            .expect_inspect_container()
            .with(eq("test-deployment"), eq(None::<InspectContainerOptions>))
            .times(1)
            .returning(move |_, _| Ok(container_inspect_response.clone()));

        // Mock mongosh command execution with auth
        mock_docker
            .expect_run_command_in_container()
            .with(
                eq("test_container_id"),
                eq(vec![
                    "mongosh".to_string(),
                    "mongodb://127.0.0.1:27017/?directConnection=true".to_string(),
                    "--username=testuser".to_string(),
                    "--password=testpass".to_string(),
                    "--eval".to_string(),
                    "db.getSiblingDB('admin').atlascli.findOne()?.uuid".to_string(),
                    "--quiet".to_string(),
                ]),
            )
            .times(1)
            .returning(|_, _| {
                Ok(CommandOutput {
                    stdout: vec!["deployment-uuid-456".to_string()],
                    stderr: vec![],
                })
            });

        let client = Client::new(mock_docker);

        // Act
        let result = client.get_deployment_id("test-deployment").await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "deployment-uuid-456");
    }

    #[tokio::test]
    async fn test_get_deployment_id_happy_path_file_auth() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let mut container_inspect_response = create_test_container_inspect_response();

        // Add username and password file environment variables
        if let Some(config) = container_inspect_response.config.as_mut()
            && let Some(env) = config.env.as_mut()
        {
            env.push("MONGODB_INITDB_ROOT_USERNAME_FILE=/run/secrets/username".to_string());
            env.push("MONGODB_INITDB_ROOT_PASSWORD_FILE=/run/secrets/password".to_string());
        }

        // Mock get_deployment call
        mock_docker
            .expect_inspect_container()
            .with(eq("test-deployment"), eq(None::<InspectContainerOptions>))
            .times(1)
            .returning(move |_, _| Ok(container_inspect_response.clone()));

        // Mock reading username file
        mock_docker
            .expect_run_command_in_container()
            .with(
                eq("test_container_id"),
                eq(vec!["cat".to_string(), "/run/secrets/username".to_string()]),
            )
            .times(1)
            .returning(|_, _| {
                Ok(CommandOutput {
                    stdout: vec!["fileuser".to_string()],
                    stderr: vec![],
                })
            });

        // Mock reading password file
        mock_docker
            .expect_run_command_in_container()
            .with(
                eq("test_container_id"),
                eq(vec!["cat".to_string(), "/run/secrets/password".to_string()]),
            )
            .times(1)
            .returning(|_, _| {
                Ok(CommandOutput {
                    stdout: vec!["filepass".to_string()],
                    stderr: vec![],
                })
            });

        // Mock mongosh command execution with file-based auth
        mock_docker
            .expect_run_command_in_container()
            .with(
                eq("test_container_id"),
                eq(vec![
                    "mongosh".to_string(),
                    "mongodb://127.0.0.1:27017/?directConnection=true".to_string(),
                    "--username=fileuser".to_string(),
                    "--password=filepass".to_string(),
                    "--eval".to_string(),
                    "db.getSiblingDB('admin').atlascli.findOne()?.uuid".to_string(),
                    "--quiet".to_string(),
                ]),
            )
            .times(1)
            .returning(|_, _| {
                Ok(CommandOutput {
                    stdout: vec!["deployment-uuid-789".to_string()],
                    stderr: vec![],
                })
            });

        let client = Client::new(mock_docker);

        // Act
        let result = client.get_deployment_id("test-deployment").await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "deployment-uuid-789");
    }

    #[tokio::test]
    async fn test_get_deployment_id_get_deployment_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();

        // Mock get_deployment call to fail
        mock_docker
            .expect_inspect_container()
            .with(
                eq("nonexistent-deployment"),
                eq(None::<InspectContainerOptions>),
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
        let result = client.get_deployment_id("nonexistent-deployment").await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            GetDeploymentIdError::GetDeployment(GetDeploymentError::ContainerInspect(_)) => {
                // Expected error
            }
            other => panic!("Expected GetDeployment error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_get_deployment_id_get_mongodb_username_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let mut container_inspect_response = create_test_container_inspect_response();

        // Add username file environment variable (but no direct username)
        if let Some(config) = container_inspect_response.config.as_mut()
            && let Some(env) = config.env.as_mut()
        {
            env.push("MONGODB_INITDB_ROOT_USERNAME_FILE=/run/secrets/username".to_string());
        }

        // Mock get_deployment call
        mock_docker
            .expect_inspect_container()
            .with(eq("test-deployment"), eq(None::<InspectContainerOptions>))
            .times(1)
            .returning(move |_, _| Ok(container_inspect_response.clone()));

        // Mock reading username file to fail
        mock_docker
            .expect_run_command_in_container()
            .with(
                eq("test_container_id"),
                eq(vec!["cat".to_string(), "/run/secrets/username".to_string()]),
            )
            .times(1)
            .returning(|_, _| {
                Err(RunCommandInContainerError::CreateExec(
                    BollardError::DockerResponseServerError {
                        status_code: 500,
                        message: "Failed to read file".to_string(),
                    },
                ))
            });

        let client = Client::new(mock_docker);

        // Act
        let result = client.get_deployment_id("test-deployment").await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            GetDeploymentIdError::GetMongodbUsername(_) => {
                // Expected error
            }
            other => panic!("Expected GetMongodbUsername error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_get_deployment_id_get_mongodb_password_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let mut container_inspect_response = create_test_container_inspect_response();

        // Add password file environment variable (but no direct password)
        if let Some(config) = container_inspect_response.config.as_mut()
            && let Some(env) = config.env.as_mut()
        {
            env.push("MONGODB_INITDB_ROOT_PASSWORD_FILE=/run/secrets/password".to_string());
        }

        // Mock get_deployment call
        mock_docker
            .expect_inspect_container()
            .with(eq("test-deployment"), eq(None::<InspectContainerOptions>))
            .times(1)
            .returning(move |_, _| Ok(container_inspect_response.clone()));

        // Mock reading password file to fail
        mock_docker
            .expect_run_command_in_container()
            .with(
                eq("test_container_id"),
                eq(vec!["cat".to_string(), "/run/secrets/password".to_string()]),
            )
            .times(1)
            .returning(|_, _| {
                Err(RunCommandInContainerError::StartExec(
                    BollardError::DockerResponseServerError {
                        status_code: 500,
                        message: "Failed to start exec".to_string(),
                    },
                ))
            });

        let client = Client::new(mock_docker);

        // Act
        let result = client.get_deployment_id("test-deployment").await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            GetDeploymentIdError::GetMongodbPassword(_) => {
                // Expected error
            }
            other => panic!("Expected GetMongodbPassword error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_get_deployment_id_run_mongosh_command_error() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let container_inspect_response = create_test_container_inspect_response();

        // Mock get_deployment call
        mock_docker
            .expect_inspect_container()
            .with(eq("test-deployment"), eq(None::<InspectContainerOptions>))
            .times(1)
            .returning(move |_, _| Ok(container_inspect_response.clone()));

        // Mock mongosh command execution to fail
        mock_docker
            .expect_run_command_in_container()
            .with(
                eq("test_container_id"),
                eq(vec![
                    "mongosh".to_string(),
                    "mongodb://127.0.0.1:27017/?directConnection=true".to_string(),
                    "--eval".to_string(),
                    "db.getSiblingDB('admin').atlascli.findOne()?.uuid".to_string(),
                    "--quiet".to_string(),
                ]),
            )
            .times(1)
            .returning(|_, _| Err(RunCommandInContainerError::GetOutput));

        let client = Client::new(mock_docker);

        // Act
        let result = client.get_deployment_id("test-deployment").await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            GetDeploymentIdError::RunMongoshCommand(_) => {
                // Expected error
            }
            other => panic!("Expected RunMongoshCommand error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_get_deployment_id_mixed_auth_env_username_file_password() {
        // Arrange
        let mut mock_docker = MockDocker::new();
        let mut container_inspect_response = create_test_container_inspect_response();

        // Add username from env and password from file
        if let Some(config) = container_inspect_response.config.as_mut()
            && let Some(env) = config.env.as_mut()
        {
            env.push("MONGODB_INITDB_ROOT_USERNAME=envuser".to_string());
            env.push("MONGODB_INITDB_ROOT_PASSWORD_FILE=/run/secrets/password".to_string());
        }

        // Mock get_deployment call
        mock_docker
            .expect_inspect_container()
            .with(eq("test-deployment"), eq(None::<InspectContainerOptions>))
            .times(1)
            .returning(move |_, _| Ok(container_inspect_response.clone()));

        // Mock reading password file
        mock_docker
            .expect_run_command_in_container()
            .with(
                eq("test_container_id"),
                eq(vec!["cat".to_string(), "/run/secrets/password".to_string()]),
            )
            .times(1)
            .returning(|_, _| {
                Ok(CommandOutput {
                    stdout: vec!["filepass".to_string()],
                    stderr: vec![],
                })
            });

        // Mock mongosh command execution with mixed auth
        mock_docker
            .expect_run_command_in_container()
            .with(
                eq("test_container_id"),
                eq(vec![
                    "mongosh".to_string(),
                    "mongodb://127.0.0.1:27017/?directConnection=true".to_string(),
                    "--username=envuser".to_string(),
                    "--password=filepass".to_string(),
                    "--eval".to_string(),
                    "db.getSiblingDB('admin').atlascli.findOne()?.uuid".to_string(),
                    "--quiet".to_string(),
                ]),
            )
            .times(1)
            .returning(|_, _| {
                Ok(CommandOutput {
                    stdout: vec!["deployment-uuid-mixed".to_string()],
                    stderr: vec![],
                })
            });

        let client = Client::new(mock_docker);

        // Act
        let result = client.get_deployment_id("test-deployment").await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "deployment-uuid-mixed");
    }

    #[tokio::test]
    async fn test_get_deployment_id_all_run_command_in_container_error_variants() {
        // Test all variants of RunCommandInContainerError to ensure full coverage

        // Test GetOutputError
        let mut mock_docker = MockDocker::new();
        let container_inspect_response = create_test_container_inspect_response();

        mock_docker
            .expect_inspect_container()
            .with(eq("test-deployment"), eq(None::<InspectContainerOptions>))
            .times(1)
            .returning(move |_, _| Ok(container_inspect_response.clone()));

        mock_docker
            .expect_run_command_in_container()
            .with(
                eq("test_container_id"),
                eq(vec![
                    "mongosh".to_string(),
                    "mongodb://127.0.0.1:27017/?directConnection=true".to_string(),
                    "--eval".to_string(),
                    "db.getSiblingDB('admin').atlascli.findOne()?.uuid".to_string(),
                    "--quiet".to_string(),
                ]),
            )
            .times(1)
            .returning(|_, _| {
                Err(RunCommandInContainerError::GetOutputError(
                    BollardError::DockerResponseServerError {
                        status_code: 500,
                        message: "Failed to get output".to_string(),
                    },
                ))
            });

        let client = Client::new(mock_docker);
        let result = client.get_deployment_id("test-deployment").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            GetDeploymentIdError::RunMongoshCommand(
                RunCommandInContainerError::GetOutputError(_),
            ) => {
                // Expected error
            }
            other => panic!(
                "Expected RunMongoshCommand GetOutputError, got: {:?}",
                other
            ),
        }
    }

    #[tokio::test]
    async fn test_get_deployment_id_empty_stdout() {
        // Test behavior when mongosh returns empty stdout
        let mut mock_docker = MockDocker::new();
        let container_inspect_response = create_test_container_inspect_response();

        mock_docker
            .expect_inspect_container()
            .with(eq("test-deployment"), eq(None::<InspectContainerOptions>))
            .times(1)
            .returning(move |_, _| Ok(container_inspect_response.clone()));

        mock_docker
            .expect_run_command_in_container()
            .with(
                eq("test_container_id"),
                eq(vec![
                    "mongosh".to_string(),
                    "mongodb://127.0.0.1:27017/?directConnection=true".to_string(),
                    "--eval".to_string(),
                    "db.getSiblingDB('admin').atlascli.findOne()?.uuid".to_string(),
                    "--quiet".to_string(),
                ]),
            )
            .times(1)
            .returning(|_, _| {
                Ok(CommandOutput {
                    stdout: vec![],
                    stderr: vec![],
                })
            });

        let client = Client::new(mock_docker);
        let result = client.get_deployment_id("test-deployment").await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_deployment_id_username_only() {
        // Test when only username is provided (no password)
        let mut mock_docker = MockDocker::new();
        let mut container_inspect_response = create_test_container_inspect_response();

        // Add only username environment variable
        if let Some(config) = container_inspect_response.config.as_mut()
            && let Some(env) = config.env.as_mut()
        {
            env.push("MONGODB_INITDB_ROOT_USERNAME=onlyuser".to_string());
        }

        mock_docker
            .expect_inspect_container()
            .with(eq("test-deployment"), eq(None::<InspectContainerOptions>))
            .times(1)
            .returning(move |_, _| Ok(container_inspect_response.clone()));

        mock_docker
            .expect_run_command_in_container()
            .with(
                eq("test_container_id"),
                eq(vec![
                    "mongosh".to_string(),
                    "mongodb://127.0.0.1:27017/?directConnection=true".to_string(),
                    "--username=onlyuser".to_string(),
                    "--eval".to_string(),
                    "db.getSiblingDB('admin').atlascli.findOne()?.uuid".to_string(),
                    "--quiet".to_string(),
                ]),
            )
            .times(1)
            .returning(|_, _| {
                Ok(CommandOutput {
                    stdout: vec!["deployment-uuid-username-only".to_string()],
                    stderr: vec![],
                })
            });

        let client = Client::new(mock_docker);
        let result = client.get_deployment_id("test-deployment").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "deployment-uuid-username-only");
    }

    #[tokio::test]
    async fn test_get_deployment_id_password_only() {
        // Test when only password is provided (no username)
        let mut mock_docker = MockDocker::new();
        let mut container_inspect_response = create_test_container_inspect_response();

        // Add only password environment variable
        if let Some(config) = container_inspect_response.config.as_mut()
            && let Some(env) = config.env.as_mut()
        {
            env.push("MONGODB_INITDB_ROOT_PASSWORD=onlypass".to_string());
        }

        mock_docker
            .expect_inspect_container()
            .with(eq("test-deployment"), eq(None::<InspectContainerOptions>))
            .times(1)
            .returning(move |_, _| Ok(container_inspect_response.clone()));

        mock_docker
            .expect_run_command_in_container()
            .with(
                eq("test_container_id"),
                eq(vec![
                    "mongosh".to_string(),
                    "mongodb://127.0.0.1:27017/?directConnection=true".to_string(),
                    "--password=onlypass".to_string(),
                    "--eval".to_string(),
                    "db.getSiblingDB('admin').atlascli.findOne()?.uuid".to_string(),
                    "--quiet".to_string(),
                ]),
            )
            .times(1)
            .returning(|_, _| {
                Ok(CommandOutput {
                    stdout: vec!["deployment-uuid-password-only".to_string()],
                    stderr: vec![],
                })
            });

        let client = Client::new(mock_docker);
        let result = client.get_deployment_id("test-deployment").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "deployment-uuid-password-only");
    }
}
