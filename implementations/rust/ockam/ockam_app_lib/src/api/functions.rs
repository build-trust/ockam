//! In this module we define the functions that are exposed to the C API.
//!
//! The main flow is expected to be like:
//!     - the frontend calls [`initialize_application`] to initialize the application state
//!       and provide a callback to receive updates and notifications
//!     - the application will receive updates and notifications through the provided callbacks
//!       right away
//!     - the frontend may call any other function to interact with the application and will
//!       receive the updates status asynchronously through the callback
//!     - the frontend calls [`shutdown_application`] to gracefully shutdown the application
//!
//!

use crate::api::state::{c, convert_runtime_information_to_c, rust};
use crate::api::{state, to_c_string};
use crate::cli::check_ockam_executable;
use crate::state::AppState;
use ockam_api::cli_state::CliState;
use ockam_api::orchestrator::email_address::EmailAddress;
use ockam_core::Address;
use std::ffi::c_char;
use std::pin::Pin;
use tracing::{error, info};

/// Global application state.
// marked as pinned because moving (or dropping) the instance would invalidate the pointers
// resulting in a crash
static mut APPLICATION_STATE: Option<Pin<Box<AppState>>> = None;

const ERROR_NOT_INITIALIZED: &str =
    "initialize_application must be called before any other function";

/// This functions initializes the application state.
/// It must be called before any other function.
/// Returns true if successful, false otherwise.
/// In case of failure the application should propose a reset to the user.
#[no_mangle]
#[allow(static_mut_refs)]
extern "C" fn initialize_application(
    // we can't use any type alias because cbindgen doesn't support them
    application_state_callback: unsafe extern "C" fn(
        state: super::state::c::ApplicationState,
    ) -> (),
    notification_callback: unsafe extern "C" fn(
        notification: super::notification::c::Notification,
    ) -> (),
) -> bool {
    unsafe {
        if APPLICATION_STATE.is_some() {
            panic!("initialize_application must be called only once")
        }
    }

    for &key in &[
        "OCKAM_CONTROLLER_ADDR",
        "OCKAM_CONTROLLER_IDENTITY_ID",
        "OCKAM_HOME",
    ] {
        match std::env::var(key) {
            Ok(value) => info!("{key} has been set: {value}"),
            Err(_) => info!("{key} is not set"),
        }
    }

    let result = AppState::new(
        super::state::rust::ApplicationStateCallback::new(application_state_callback),
        super::notification::rust::NotificationCallback::new(notification_callback),
    );

    let app_state = match result {
        Ok(app_state) => app_state,
        Err(err) => {
            eprintln!("Failed to load the local Ockam configuration: {err}");
            return false;
        }
    };
    app_state.setup_logging_tracing();

    #[cfg(target_os = "macos")]
    crate::cli::add_homebrew_to_path();

    if let Err(err) = check_ockam_executable() {
        error!(?err, "Couldn't find the ockam executable");
        app_state.notify(super::notification::rust::Notification {
            kind: super::notification::rust::Kind::Error,
            title: "Couldn't find the ockam executable".to_string(),
            message: "Please install ockam and make sure it is in your PATH".to_string(),
        });
        //sleep for a bit to allow time for the notification to be displayed
        std::thread::sleep(std::time::Duration::from_secs(1));
        std::process::exit(1);
    }
    unsafe {
        APPLICATION_STATE.replace(Box::pin(app_state));
    }

    // avoid waiting for the load to return for a quicker initialization
    let app_state = unsafe { APPLICATION_STATE.as_ref() }.expect(ERROR_NOT_INITIALIZED);
    app_state.context().runtime().spawn(async move {
        app_state.publish_state().await;
        app_state.load_model_state().await;
    });

    true
}

/// Accept the invitation with the provided id.
#[no_mangle]
#[allow(static_mut_refs)]
extern "C" fn accept_invitation(id: *const c_char) {
    let id = unsafe { std::ffi::CStr::from_ptr(id).to_str().unwrap().to_string() };
    let app_state = unsafe { APPLICATION_STATE.as_ref() }.expect(ERROR_NOT_INITIALIZED);
    app_state.context().runtime().spawn(async {
        let result = app_state.accept_invitation(id).await;
        if let Err(err) = result {
            error!(?err, "Couldn't accept the invitation");
        }
    });
}

/// Ignore the invitation with the provided id.
#[no_mangle]
#[allow(static_mut_refs)]
extern "C" fn ignore_invitation(id: *const c_char) {
    let id = unsafe { std::ffi::CStr::from_ptr(id).to_str().unwrap().to_string() };
    let app_state = unsafe { APPLICATION_STATE.as_ref() }.expect(ERROR_NOT_INITIALIZED);
    app_state.context().runtime().spawn(async {
        let result = app_state.ignore_invitation(id).await;
        if let Err(err) = result {
            error!(?err, "Couldn't accept the invitation");
        }
    });
}

/// Initiate graceful shutdown of the application, exit process when complete.
#[no_mangle]
#[allow(static_mut_refs)]
extern "C" fn shutdown_application() {
    let app_state = unsafe { APPLICATION_STATE.as_ref() };
    if let Some(app_state) = app_state {
        app_state.shutdown();
    } else {
        std::process::exit(0);
    }
}

/// Share a local service with the provided emails.
/// Emails are separated by ';'.
#[no_mangle]
#[allow(static_mut_refs)]
extern "C" fn share_local_service(name: *const c_char, emails: *const c_char) -> *const c_char {
    let worker_addr = unsafe { std::ffi::CStr::from_ptr(name).to_str().unwrap().to_string() };
    let worker_addr: Address = worker_addr.into();
    let emails: Vec<String> = unsafe {
        std::ffi::CStr::from_ptr(emails)
            .to_str()
            .unwrap()
            .to_string()
    }
    .split(';')
    .map(|s| s.to_string())
    .filter(|s| !s.is_empty())
    .collect();

    let app_state = unsafe { APPLICATION_STATE.as_ref() }.expect(ERROR_NOT_INITIALIZED);
    let result = app_state.context().runtime().block_on(async {
        let mut result = Ok(());
        for email in emails {
            match EmailAddress::parse(&email) {
                Ok(email_address) => {
                    result = app_state
                        .create_service_invitation_by_alias(
                            &app_state.context(),
                            email_address,
                            &worker_addr,
                        )
                        .await;
                }
                Err(e) => {
                    error!("the email address {email} is not a valid email address");
                    result = Err(e.to_string())
                }
            }
            app_state.publish_state().await;
            if result.is_err() {
                break;
            }
        }
        result
    });

    match result {
        Ok(_) => std::ptr::null(),
        Err(err) => to_c_string(err.to_string()),
    }
}

/// Enable an accepted service associated with the invite id.
#[no_mangle]
#[allow(static_mut_refs)]
extern "C" fn enable_accepted_service(invitation_id: *const c_char) {
    let invitation_id = unsafe {
        std::ffi::CStr::from_ptr(invitation_id)
            .to_str()
            .unwrap()
            .to_string()
    };
    let app_state = unsafe { APPLICATION_STATE.as_ref() }.expect(ERROR_NOT_INITIALIZED);
    app_state.context().runtime().spawn(async move {
        let result = app_state.enable_tcp_inlet(&invitation_id).await;
        if let Err(err) = result {
            error!(?err, "Couldn't enable the service");
        }
    });
}

/// Disable an accepted service associated with the invite id.
#[no_mangle]
#[allow(static_mut_refs)]
extern "C" fn disable_accepted_service(invitation_id: *const c_char) {
    let invitation_id = unsafe {
        std::ffi::CStr::from_ptr(invitation_id)
            .to_str()
            .unwrap()
            .to_string()
    };
    let app_state = unsafe { APPLICATION_STATE.as_ref() }.expect(ERROR_NOT_INITIALIZED);
    app_state.context().runtime().spawn(async move {
        let result = app_state.disable_tcp_inlet(&invitation_id).await;
        if let Err(err) = result {
            error!(?err, "Couldn't disable the service");
        }
    });
}

/// Removes a local service with the provided name.
#[no_mangle]
#[allow(static_mut_refs)]
extern "C" fn delete_local_service(worker_addr: *const c_char) {
    let worker_addr = unsafe {
        std::ffi::CStr::from_ptr(worker_addr)
            .to_str()
            .unwrap()
            .to_string()
    };
    let app_state = unsafe { APPLICATION_STATE.as_ref() }.expect(ERROR_NOT_INITIALIZED);
    app_state.context().runtime().spawn(async {
        let result = app_state.tcp_outlet_delete(worker_addr.into()).await;
        if let Err(err) = result {
            error!(?err, "Couldn't delete the local service");
        }
    });
}

/// Creates a local service with the provided name and address.
/// Returns null if successful, otherwise returns an error message.
#[no_mangle]
#[allow(static_mut_refs)]
extern "C" fn create_local_service(
    worker_addr: *const c_char,
    address: *const c_char,
) -> *const c_char {
    let worker_addr = unsafe {
        std::ffi::CStr::from_ptr(worker_addr)
            .to_str()
            .unwrap()
            .to_string()
    };
    let socket_addr = unsafe {
        std::ffi::CStr::from_ptr(address)
            .to_str()
            .unwrap()
            .to_string()
    };

    let app_state = unsafe { APPLICATION_STATE.as_ref() }.expect(ERROR_NOT_INITIALIZED);
    let result = app_state.context().runtime().block_on(async {
        let result = app_state.tcp_outlet_create(worker_addr, socket_addr).await;
        app_state.publish_state().await;
        result
    });

    match result {
        Ok(_) => std::ptr::null(),
        Err(err) => to_c_string(format!("{}", err)),
    }
}

/// Synchronously resets the application state to a fresh installation.
/// A restart is **required** afterward.
#[no_mangle]
#[allow(static_mut_refs)]
extern "C" fn reset_application_state() {
    let app_state = unsafe { APPLICATION_STATE.as_ref() };
    match app_state {
        Some(app_state) => {
            app_state.context().runtime().block_on(async move {
                let result = app_state.reset().await;
                if let Err(err) = result {
                    error!(?err, "Cannot reset the application state");
                }
            });
        }
        None => {
            let app_state = unsafe { APPLICATION_STATE.as_ref() }.expect(ERROR_NOT_INITIALIZED);

            app_state.context().runtime().block_on(async {
                // allow disk state reset even if we don't have an application state
                CliState::backup_and_reset().await.expect(
                "Failed to initialize CliState. Try to manually remove the '~/.ockam' directory",
            );
            })
        }
    }
}

/// Starts user enrollment
#[no_mangle]
#[allow(static_mut_refs)]
extern "C" fn enroll_user() {
    let app_state = unsafe { APPLICATION_STATE.as_ref() }.expect(ERROR_NOT_INITIALIZED);

    app_state
        .context()
        .runtime()
        .spawn(async move { app_state.enroll_user().await });
}

/// This function retrieve the current version of the application state, for polling purposes.
#[no_mangle]
#[allow(static_mut_refs)]
extern "C" fn application_state_snapshot() -> super::state::c::ApplicationState {
    let app_state = unsafe { APPLICATION_STATE.as_ref() }.expect(ERROR_NOT_INITIALIZED);

    let public_rust_state = app_state
        .context()
        .runtime()
        .block_on(async { app_state.snapshot().await })
        .expect("Cannot retrieve application state");

    super::state::convert_application_state_to_c(public_rust_state)
}

/// This functions returns runtime information about the application.
/// It is used to display the version and the git hash of the application.
#[no_mangle]
extern "C" fn runtime_information() -> c::RuntimeInformation {
    let home = std::env::var("OCKAM_HOME").ok();
    let controller_addr = std::env::var("OCKAM_CONTROLLER_ADDR").ok();
    let controller_identity = std::env::var("OCKAM_CONTROLLER_IDENTITY_ID").ok();

    let info = rust::RuntimeInformation {
        version: env!("CARGO_PKG_VERSION").to_string(),
        commit: env!("GIT_HASH").to_string(),
        home,
        controller_addr,
        controller_identity,
    };

    convert_runtime_information_to_c(info)
}

/// Free the runtime information memory
#[no_mangle]
extern "C" fn free_runtime_information(information: c::RuntimeInformation) {
    state::free_runtime_information_free(information);
}
