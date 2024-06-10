use httpmock::Method::GET;
use httpmock::MockServer;
use serde_json::json;

use crate::io::tests::TestWriter;

use super::*;

fn get_repositories(url: String) -> Repositories {
    Repositories {
        packagist: Packagist {
            _type: "composer".to_owned(),
            url,
        },
    }
}

#[tokio::test]
async fn simple() {
    let server = MockServer::start();

    let hello_mock = server.mock(|when, then| {
        when.method(GET).path("/p2/foo/bar.json");
        then.status(200).json_body(json!({
            "packages" : {
                "foo/bar" : [{
                    "name" : "foo/bar",
                    "version" : "1.2.3",
                    "version_normalized": "1.2.3.0",
                }]
            }
        }));
    });

    let mut composer = Composer {
        require: Some({
            let mut map = IndexMap::new();
            map.insert("foo/bar".to_owned(), "1.2.3".to_owned());
            map
        }),
        repositories: Some(get_repositories(server.base_url())),
    };
    let mut stderr = TestWriter::new();
    let lock = composer.get_lock(&mut stderr).await.unwrap();
    hello_mock.assert();
    let version = &lock.packages[0];
    assert_eq!(version.version, "1.2.3".to_owned());
    assert!(stderr.output().is_empty())
}

#[tokio::test]
async fn one_depend() {
    let server = MockServer::start();

    let bar = server.mock(|when, then| {
        when.method(GET).path("/p2/foo/bar.json");
        then.status(200).json_body(json!({
            "packages" : {
                "foo/bar" : [{
                    "name" : "foo/bar",
                    "version" : "1.2.3",
                    "version_normalized": "1.2.3.0",
                    "require":{
                        "foo2/bar2" : "2.3.0",
                    }
                }]
            }
        }));
    });
    let bar2 = server.mock(|when, then| {
        when.method(GET).path("/p2/foo2/bar2.json");
        then.status(200).json_body(json!({
            "packages" : {
                "foo2/bar2" : [{
                    "name" : "foo/bar",
                    "version" : "2.3.0",
                    "version_normalized": "2.3.0.0",
                }]
            }
        }));
    });

    let mut composer = Composer {
        require: Some({
            let mut map = IndexMap::new();
            map.insert("foo/bar".to_owned(), "1.2.3".to_owned());
            map
        }),
        repositories: Some(get_repositories(server.base_url())),
    };
    let mut stderr = TestWriter::new();
    let lock = composer.get_lock(&mut stderr).await.unwrap();
    bar.assert();
    bar2.assert();
    let version = &lock.packages[0];
    assert_eq!(version.version, "1.2.3".to_owned());

    let bar2_version = &lock.packages[1];
    assert_eq!(bar2_version.version, "2.3.0".to_owned());

    assert!(stderr.output().is_empty())
}

#[tokio::test]
async fn last_stable() {
    let server = MockServer::start();

    let hello_mock = server.mock(|when, then| {
        when.method(GET).path("/p2/foo/bar.json");
        then.status(200).json_body(json!({
            "packages" : {
                "foo/bar" : [{
                    "name" : "foo/bar",
                    "version" : "1.3.0-rc1",
                    "version_normalized": "1.3.0.1",
                },{
                    "name" : "foo/bar",
                    "version" : "1.2.3",
                    "version_normalized": "1.2.3.0",
                }]
            }
        }));
    });

    let mut composer = Composer {
        require: Some({
            let mut map = IndexMap::new();
            map.insert("foo/bar".to_owned(), "*".to_owned());
            map
        }),
        repositories: Some(get_repositories(server.base_url())),
    };
    let mut stderr = TestWriter::new();
    let lock = composer.get_lock(&mut stderr).await.unwrap();
    hello_mock.assert();
    let version = &lock.packages[0];
    assert_eq!(version.version, "1.2.3".to_owned());
    assert!(stderr.output().is_empty())
}
