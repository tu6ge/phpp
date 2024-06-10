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
fn default_context(composer: &Composer) -> Arc<Mutex<Context>> {
    let p2_url = composer.get_package_url().unwrap();
    let mut context = Context::new().unwrap();
    context.p2_url = p2_url;
    Arc::new(Mutex::new(context))
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
    let ctx = default_context(&composer);

    let lock = composer.get_lock(&mut stderr, ctx).await.unwrap();
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
    let ctx = default_context(&composer);

    let lock = composer.get_lock(&mut stderr, ctx).await.unwrap();
    bar.assert();
    bar2.assert();
    assert_eq!(lock.packages.len(), 2);
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
    let ctx = default_context(&composer);

    let lock = composer.get_lock(&mut stderr, ctx).await.unwrap();
    hello_mock.assert();
    let version = &lock.packages[0];
    assert_eq!(version.version, "1.2.3".to_owned());
    assert!(stderr.output().is_empty())
}

#[tokio::test]
async fn php_version() {
    let server = MockServer::start();

    let hello_mock = server.mock(|when, then| {
        when.method(GET).path("/p2/foo/bar.json");
        then.status(200).json_body(json!({
            "packages" : {
                "foo/bar" : [{
                    "name" : "foo/bar",
                    "version" : "1.2.3",
                    "version_normalized": "1.2.3.0",
                    "require":{
                      "php": ">=8.3.0",
                    }
                },{
                    "name" : "foo/bar",
                    "version" : "1.1.0",
                    "version_normalized": "1.1.0.0",
                    "require":{
                      "php": ">=8.0.0",
                    }
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
    let p2_url = composer.get_package_url().unwrap();
    let mut context = Context::new().unwrap();
    context.p2_url = p2_url;
    context.php_version = "8.2.0".to_owned();
    let ctx = Arc::new(Mutex::new(context));

    let error = composer.get_lock(&mut stderr, ctx).await.unwrap_err();
    assert!(matches!(error, ComposerError::PhpVersion));
    hello_mock.assert();

    assert_eq!(
        stderr.output(),
        "foo/bar(*) -> .. -> foo/bar(1.2.3) need PHP version is >=8.3.0"
    );

    // let mut composer = Composer {
    //     require: Some({
    //         let mut map = IndexMap::new();
    //         map.insert("foo/bar".to_owned(), "1.1.0".to_owned());
    //         map
    //     }),
    //     repositories: Some(get_repositories(server.base_url())),
    // };
    // let mut stderr = TestWriter::new();
    // let p2_url = composer.get_package_url().unwrap();
    // let mut context = Context::new().unwrap();
    // context.p2_url = p2_url;
    // context.php_version = "8.2.0".to_owned();
    // let ctx = Arc::new(Mutex::new(context));

    // let _ = composer.get_lock(&mut stderr, ctx).await.unwrap();

    // hello_mock.assert();

    // assert!(stderr.output().is_empty());
}

#[tokio::test]
async fn php_extensions() {
    let server = MockServer::start();

    let hello_mock = server.mock(|when, then| {
        when.method(GET).path("/p2/foo/bar.json");
        then.status(200).json_body(json!({
            "packages" : {
                "foo/bar" : [{
                    "name" : "foo/bar",
                    "version" : "1.2.3",
                    "version_normalized": "1.2.3.0",
                    "require":{
                      "ext-dom": "*",
                    }
                },]
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
    let p2_url = composer.get_package_url().unwrap();
    let mut context = Context::new().unwrap();
    context.p2_url = p2_url;
    context.php_extensions = vec![];
    let ctx = Arc::new(Mutex::new(context));

    let error = composer.get_lock(&mut stderr, ctx).await.unwrap_err();
    assert!(matches!(error, ComposerError::PhpVersion));
    hello_mock.assert();

    assert_eq!(
        stderr.output(),
        "foo/bar(*) -> .. -> foo/bar(1.2.3) need ext-dom,it is missing from your system. Install or enable PHP's dom extension."
    );
}

#[tokio::test]
async fn auto_choise_version() {
    let server = MockServer::start();

    let hello_mock = server.mock(|when, then| {
        when.method(GET).path("/p2/foo/bar.json");
        then.status(200).json_body(json!({
            "packages" : {
                "foo/bar" : [{
                    "name" : "foo/bar",
                    "version" : "2.2.3",
                    "version_normalized": "2.2.3.0",
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
            map.insert("foo/bar".to_owned(), "^1".to_owned());
            map
        }),
        repositories: Some(get_repositories(server.base_url())),
    };
    let mut stderr = TestWriter::new();
    let ctx = default_context(&composer);

    let lock = composer.get_lock(&mut stderr, ctx).await.unwrap();
    hello_mock.assert();
    assert_eq!(lock.packages.len(), 1);
    let version = &lock.packages[0];
    assert_eq!(version.version, "1.2.3".to_owned());
    assert!(stderr.output().is_empty())
}
