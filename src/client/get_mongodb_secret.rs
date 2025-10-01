use crate::{
    docker::{RunCommandInContainer, RunCommandInContainerError},
    models::Deployment,
};

pub async fn get_mongodb_secret<D: RunCommandInContainer>(
    docker: &D,
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
        let command_output = docker
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
