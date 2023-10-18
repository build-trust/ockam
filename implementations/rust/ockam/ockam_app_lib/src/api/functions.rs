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

use crate::api::to_c_string;
use crate::cli::check_ockam_executable;
use crate::state::AppState;
use libc::c_char;
use tracing::{error, info};

/// Global application state.
static mut APPLICATION_STATE: Option<AppState> = None;

const ERROR_NOT_INITIALIZED: &str =
    "initialize_application must be called before any other function";

/// This functions initializes the application state.
/// It must be called before any other function.
#[no_mangle]
extern "C" fn initialize_application(
    // we can't use any type alias because cbindgen doesn't support them
    application_state_callback: unsafe extern "C" fn(
        state: super::state::c::ApplicationState,
    ) -> (),
    notification_callback: unsafe extern "C" fn(
        notification: super::notification::c::Notification,
    ) -> (),
) {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_ansi(false)
        .init();

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

    let app_state = AppState::new(
        super::state::rust::ApplicationStateCallback::new(application_state_callback),
        super::notification::rust::NotificationCallback::new(notification_callback),
    );

    if let Err(err) = check_ockam_executable() {
        error!(?err, "Couldn't find the ockam executable");
        app_state.notify(super::notification::rust::Notification {
            kind: super::notification::rust::Kind::Error,
            title: "Couldn't find the ockam executable".to_string(),
            message: "Please install ockam and make sure it is in your PATH".to_string(),
        });
        std::process::exit(1);
    }
    unsafe {
        APPLICATION_STATE.replace(app_state);
    }

    // avoid waiting for the load to return for a quicker initialization
    let app_state = unsafe { APPLICATION_STATE.as_ref().expect(ERROR_NOT_INITIALIZED) };
    app_state.context().runtime().spawn(async {
        app_state.publish_state().await;
        app_state.load_model_state().await;
    });
}

/// Accept the invitation with the provided id.
#[no_mangle]
extern "C" fn accept_invitation(id: *const c_char) {
    let id = unsafe { std::ffi::CStr::from_ptr(id).to_str().unwrap().to_string() };
    let app_state = unsafe { APPLICATION_STATE.as_ref() }.unwrap();
    app_state.context().runtime().spawn(async {
        let result = app_state.accept_invitation(id).await;
        if let Err(err) = result {
            error!(?err, "Couldn't accept the invitation");
        }
    });
}

/// Initiate graceful shutdown of the application, exit process when complete.
#[no_mangle]
extern "C" fn shutdown_application() {
    let app_state = unsafe { APPLICATION_STATE.take() };
    if let Some(app_state) = app_state {
        app_state.shutdown();
    } else {
        std::process::exit(0);
    }
}

/// Share a local service with the provided emails.
/// Emails are separated by ';'.
#[no_mangle]
extern "C" fn share_local_service(name: *const c_char, emails: *const c_char) -> *const c_char {
    let name = unsafe { std::ffi::CStr::from_ptr(name).to_str().unwrap().to_string() };
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

    let app_state = unsafe { APPLICATION_STATE.as_ref() }.unwrap();
    let result = app_state.context().runtime().block_on(async {
        let mut result = Ok(());
        for email in emails {
            result = app_state
                .create_service_invitation_by_alias(email, &name)
                .await;
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
extern "C" fn enable_accepted_service(invitation_id: *const c_char) {
    let invitation_id = unsafe {
        std::ffi::CStr::from_ptr(invitation_id)
            .to_str()
            .unwrap()
            .to_string()
    };
    let app_state = unsafe { APPLICATION_STATE.as_ref() }.unwrap();
    app_state.context().runtime().spawn(async move {
        let result = app_state.enable_tcp_inlet(&invitation_id).await;
        if let Err(err) = result {
            error!(?err, "Couldn't enable the service");
        }
    });
}

/// Disable an accepted service associated with the invite id.
#[no_mangle]
extern "C" fn disable_accepted_service(invitation_id: *const c_char) {
    let invitation_id = unsafe {
        std::ffi::CStr::from_ptr(invitation_id)
            .to_str()
            .unwrap()
            .to_string()
    };
    let app_state = unsafe { APPLICATION_STATE.as_ref() }.unwrap();
    app_state.context().runtime().spawn(async move {
        let result = app_state.disconnect_tcp_inlet(&invitation_id).await;
        if let Err(err) = result {
            error!(?err, "Couldn't disable the service");
        }
    });
}

/// Removes a local service with the provided name.
#[no_mangle]
extern "C" fn delete_local_service(name: *const c_char) {
    let name = unsafe { std::ffi::CStr::from_ptr(name).to_str().unwrap().to_string() };
    let app_state = unsafe { APPLICATION_STATE.as_ref() }.unwrap();
    app_state.context().runtime().spawn(async {
        let result = app_state.tcp_outlet_delete(name).await;
        if let Err(err) = result {
            error!(?err, "Couldn't delete the local service");
        }
    });
}

/// Creates a local service with the provided name and address.
/// Emails are separated by ';'.
/// Returns null if successful, otherwise returns an error message.
#[no_mangle]
extern "C" fn create_local_service(
    name: *const c_char,
    address: *const c_char,
    emails: *const c_char,
) -> *const c_char {
    let name = unsafe { std::ffi::CStr::from_ptr(name).to_str().unwrap().to_string() };
    let address = unsafe {
        std::ffi::CStr::from_ptr(address)
            .to_str()
            .unwrap()
            .to_string()
    };
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

    let app_state = unsafe { APPLICATION_STATE.as_ref() }.unwrap();
    let result = app_state.context().runtime().block_on(async {
        let result = app_state.tcp_outlet_create(name, address, emails).await;
        app_state.publish_state().await;
        result
    });

    match result {
        Ok(_) => std::ptr::null(),
        Err(err) => to_c_string(format!("{}", err)),
    }
}

/// Resets the application state to a fresh installation.
#[no_mangle]
extern "C" fn reset_application_state() {
    let app_state = unsafe { APPLICATION_STATE.as_ref() }.unwrap();
    app_state.context().runtime().spawn(async {
        let result = app_state.reset().await;
        if let Err(err) = result {
            error!(?err, "Cannot reset the application state");
        }
    });
}

/// Starts user enrollment
#[no_mangle]
extern "C" fn enroll_user() {
    let app_state = unsafe { APPLICATION_STATE.as_ref().expect(ERROR_NOT_INITIALIZED) };

    app_state
        .context()
        .runtime()
        .spawn(async move { app_state.enroll_user().await });
}

/// This function retrieve the current version of the application state, for polling purposes.
#[no_mangle]
extern "C" fn application_state_snapshot() -> super::state::c::ApplicationState {
    let app_state = unsafe { APPLICATION_STATE.as_ref().expect(ERROR_NOT_INITIALIZED) };

    let public_rust_state = app_state
        .context()
        .runtime()
        .block_on(async { app_state.snapshot().await })
        .expect("Cannot retrieve application state");

    super::state::convert_to_c(public_rust_state)
}
