use ockam_core::{
    async_trait, route, AddressMetadata, DenyAll, Mailbox, Mailboxes, Processor, Result,
};
use ockam_node::{Context, NullWorker, ProcessorBuilder, WorkerBuilder};
use std::string::ToString;
use std::sync::Arc;

struct NullProcessor;

#[async_trait]
impl Processor for NullProcessor {
    type Context = Context;

    async fn process(&mut self, _ctx: &mut Context) -> Result<bool> {
        tokio::task::yield_now().await;
        Ok(true)
    }
}

#[ockam_macros::test]
async fn find_terminal_for_processor(context: &mut Context) -> Result<()> {
    ProcessorBuilder::new(NullProcessor {})
        .with_address("simple_processor")
        .start(context)?;

    assert!(context
        .find_terminal_address(route!["simple_processor", "non-existing"].iter())?
        .is_none());

    ProcessorBuilder::new(NullProcessor {})
        .with_terminal_address("terminal_processor")
        .start(context)?;

    assert_eq!(
        context
            .find_terminal_address(
                route!["simple_worker", "terminal_processor", "non-existing"].iter()
            )?
            .unwrap()
            .0,
        &"terminal_processor".into()
    );

    Ok(())
}

#[ockam_macros::test]
async fn find_terminal_for_processor_alias(context: &mut Context) -> Result<()> {
    ProcessorBuilder::new(NullProcessor {})
        .with_mailboxes(Mailboxes::new(
            Mailbox::new("main", None, Arc::new(DenyAll), Arc::new(DenyAll)),
            vec![Mailbox::new(
                "alias",
                Some(AddressMetadata {
                    is_terminal: true,
                    attributes: vec![],
                }),
                Arc::new(DenyAll),
                Arc::new(DenyAll),
            )],
        ))
        .start(context)?;

    assert!(context
        .find_terminal_address(route!["main", "non-existing"].iter())?
        .is_none());

    assert_eq!(
        context
            .find_terminal_address(route!["main", "alias", "other"].iter())?
            .unwrap()
            .0,
        &"alias".into()
    );

    context.stop_address(&"main".into())?;
    ockam_node::compat::tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    assert!(context
        .find_terminal_address(route!["main", "alias", "other"].iter())?
        .is_none());

    Ok(())
}

#[ockam_macros::test]
async fn provide_and_read_processor_address_metadata(context: &mut Context) -> Result<()> {
    ProcessorBuilder::new(NullProcessor {})
        .with_address("processor_address")
        .with_metadata_attribute("TEST_KEY", "TEST_VALUE")
        .with_metadata_attribute("TEST_KEY_2", "TEST_VALUE_2")
        .start(context)?;

    let meta = context.get_metadata(&"processor_address".into())?.unwrap();

    assert!(!meta.is_terminal);

    assert_eq!(
        meta.attributes,
        vec![
            ("TEST_KEY".to_string(), "TEST_VALUE".to_string()),
            ("TEST_KEY_2".to_string(), "TEST_VALUE_2".to_string())
        ]
    );

    assert_eq!(context.get_metadata(&"non-existing-worker".into())?, None);

    context.stop_address(&"processor_address".into())?;
    ockam_node::compat::tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    assert_eq!(context.get_metadata(&"processor_address".into())?, None);

    Ok(())
}

#[ockam_macros::test]
async fn find_terminal_for_worker(context: &mut Context) -> Result<()> {
    WorkerBuilder::new(NullWorker {})
        .with_address("simple_worker")
        .start(context)?;

    assert!(context
        .find_terminal_address(route!["simple_worker", "non-existing"].iter())?
        .is_none());

    WorkerBuilder::new(NullWorker {})
        .with_terminal_address("terminal_worker")
        .start(context)?;

    assert_eq!(
        context
            .find_terminal_address(
                route!["simple_worker", "terminal_worker", "non-existing"].iter()
            )?
            .unwrap()
            .0,
        &"terminal_worker".into()
    );

    context.stop_address(&"terminal_worker".into())?;
    assert_eq!(
        context.find_terminal_address(route!["terminal_worker"].iter())?,
        None
    );

    Ok(())
}

#[ockam_macros::test]
async fn find_terminal_for_worker_alias(context: &mut Context) -> Result<()> {
    WorkerBuilder::new(NullWorker {})
        .with_mailboxes(Mailboxes::new(
            Mailbox::new("main", None, Arc::new(DenyAll), Arc::new(DenyAll)),
            vec![Mailbox::new(
                "alias",
                Some(AddressMetadata {
                    is_terminal: true,
                    attributes: vec![],
                }),
                Arc::new(DenyAll),
                Arc::new(DenyAll),
            )],
        ))
        .start(context)?;

    assert!(context
        .find_terminal_address(route!["main", "non-existing"].iter())?
        .is_none());

    assert_eq!(
        context
            .find_terminal_address(route!["main", "alias", "other"].iter())?
            .unwrap()
            .0,
        &"alias".into()
    );

    context.stop_address(&"main".into())?;
    ockam_node::compat::tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    assert!(context
        .find_terminal_address(route!["main", "alias", "other"].iter())?
        .is_none());

    Ok(())
}

#[ockam_macros::test]
async fn provide_and_read_address_metadata(context: &mut Context) -> Result<()> {
    WorkerBuilder::new(NullWorker {})
        .with_address("worker_address")
        .with_metadata_attribute("TEST_KEY", "TEST_VALUE")
        .with_metadata_attribute("TEST_KEY_2", "TEST_VALUE_2")
        .start(context)?;

    let meta = context.get_metadata(&"worker_address".into())?.unwrap();

    assert!(!meta.is_terminal);

    assert_eq!(
        meta.attributes,
        vec![
            ("TEST_KEY".to_string(), "TEST_VALUE".to_string()),
            ("TEST_KEY_2".to_string(), "TEST_VALUE_2".to_string())
        ]
    );

    assert_eq!(context.get_metadata(&"non-existing-worker".into())?, None);

    context.stop_address(&"worker_address".into())?;
    ockam_node::compat::tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    assert_eq!(context.get_metadata(&"worker_address".into())?, None);

    Ok(())
}

#[ockam_macros::test]
async fn provide_and_read_address_metadata_worker_alias(context: &mut Context) -> Result<()> {
    WorkerBuilder::new(NullWorker {})
        .with_mailboxes(Mailboxes::new(
            Mailbox::new(
                "main",
                Some(AddressMetadata {
                    is_terminal: false,
                    attributes: vec![("TEST_KEY".to_string(), "TEST_VALUE".to_string())],
                }),
                Arc::new(DenyAll),
                Arc::new(DenyAll),
            ),
            vec![Mailbox::new(
                "alias",
                Some(AddressMetadata {
                    is_terminal: false,
                    attributes: vec![("TEST_KEY_2".to_string(), "TEST_VALUE_2".to_string())],
                }),
                Arc::new(DenyAll),
                Arc::new(DenyAll),
            )],
        ))
        .start(context)?;

    let meta = context.get_metadata(&"alias".into())?.unwrap();

    assert!(!meta.is_terminal);

    assert_eq!(
        meta.attributes,
        vec![("TEST_KEY_2".to_string(), "TEST_VALUE_2".to_string())]
    );

    context.stop_address(&"main".into())?;
    ockam_node::compat::tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    assert_eq!(context.get_metadata(&"alias".into())?, None);

    Ok(())
}
