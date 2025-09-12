#![cfg(feature = "e2e-tests")]

#[tokio::test(flavor = "multi_thread")]
async fn test_e2e_smoke_test_inside_docker() {
    // Check if we are running inside a docker container
    // Check can be done by looking for the /.dockerenv file
    if std::path::Path::new("/.dockerenv").exists() {
        println!("Running inside a docker container");
    } else {
        println!("Not running inside a docker container");
    }

    panic!("Remove me when finished debugging!")
}