use serde::{Deserialize, Serialize};

api_obj_impl!(LoginAttempt,
              "expiresAt" => expires_at: String,
              "status" => status: String,
              "sessionToken" => session_token: String,
              "_embedded" => embedded: Subuser,
              "_links" => links: TokenLink
);

api_obj_impl!(Subuser,
              "user" => user: User
);

api_obj_impl!(TokenLink,
              "cancel" => cancel: Link
);

api_obj_impl!(User,
              "id" => id: String,
              "status" => status: Option<String>,
              "created" => created: Option<String>,
              "activated" => activated: Option<String>,
              "statusChanged" => status_changed: Option<String>,
              "lastLogin" => last_login: Option<String>,
              "lastUpdated" => last_updated: Option<String>,
              "passwordChanged" => password_changed: Option<String>,
              "type" => xtype: Option<UsersId>,
              "profile" => profile: Option<UsersProfile>,
              "credentials" => credentials: Option<UsersInGroupCredentials>,
              "_links" => links: Option<UserLinks>
);

api_obj_impl!(UserLinks,
              "suspend" => suspend: Link,
              "schema" => schema: Link,
              "resetPassword" => reset_password: Link,
              "forgotPassword" => forgot_password: Link,
              "expirePassword" => expire_password: Link,
              "changeRecoveryQuestion" => change_recovery_question: Link,
              "self" => xself: Link,
              "type" => xtype: Link,
              "changePassword" => change_password: Link,
              "deactivate" => deactivate: Link
);

/*
 * Used for extracting group user information from calling
 * https://dev-url.okta.com/api/v1/groups/{group_id}/users
 */
api_obj_impl!(UsersInGroup,
              "id" => id: String,
              "status" => status: String,
              "created" => created: String,
              "activated" => activated: Option<String>,
              "statusChanged" => status_changed: String,
              "lastLogin" => last_login: String,
              "lastUpdated" => last_updated: String,
              "passwordChanged" => password_changed: String,
              "type" => xtype: UsersId,
              "profile" => profile: UsersProfile,
              "credentials" => credentials: UsersInGroupCredentials,
              "_links" => links: UsersInGroupLinks
);

api_obj_impl!(UsersId,
              "id" => id: String
);

api_obj_impl!(UsersProfile,
              "firstName" => first_name: String,
              "lastName" => last_name: String,
              "mobilePhone" => mobile_phone: Option<String>,
              "secondEmail" => second_email: Option<String>,
              "login" => login: String,
              "email" => email: Option<String>,
              "locale" => locale: Option<String>,
              "timeZone" => time_zone: Option<String>
);

api_obj_impl!(UsersInGroupCredentials,
              "password" => password: Option<EmptyPassword>,
              "emails" => emails: Vec<UsersInGroupEmail>,
              "provider" => provider: UsersInGroupProvider
);

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EmptyPassword {}

api_obj_impl!(UsersInGroupEmail,
              "value" => value: String,
              "status" => status: String,
              "type" => xtype: String
);

api_obj_impl!(UsersInGroupProvider,
              "type" => xtype: String,
              "name" => name: String
);

api_obj_impl!(UsersInGroupLinks,
              "self" => inner: Link
);

/*
 * Used for extracting group information from calling
 * https://dev-url.okta.com/api/v1/groups
 */
api_obj_impl!(Group,
              "id" => id: String,
              "created" => created: String,
              "lastUpdated" => last_updated: String,
              "lastMembershipUpdated" => last_membership_updated: String,
              "objectClass" => object_class: Vec<String>,
              "type" => xtype: String,
              "profile" => profile: GroupProfile,
              "_links" => links: GroupLinks
);

api_obj_impl!(GroupProfile,
              "name" => name: String,
              "description" => description: String
);

api_obj_impl!(GroupLinks,
              "logo" => logo: Vec<GroupLogo>,
              "users" => users: Link,
              "apps" => apps: Link
);

api_obj_impl!(GroupLogo,
              "name" => name: String,
              "href" => href: String,
              "type" => xtype: String
);

api_obj_impl!(Link,
              "href" => href: String,
              "method" => method: Option<String>,
              "hints" => hints: Option<Hints>
);

api_obj_impl!(Hints,
              "allow" => allow: Vec<String>
);

// Responses from checking for the session token
// https://developer.okta.com/docs/reference/api/oidc/#introspect
api_obj_impl!(TokenCheck,
              "active" => active: bool,
              "token_type" => token_type: Option<String>,
              "scope" => scope: Option<String>,
              "client_id" => client_id: Option<String>,
              "username" => username: Option<String>,
              "exp" => expires: Option<usize>,
              "sub" => sub_email: Option<String>,
              "device_id" => device_id: Option<String>
);

// Response from client credentials flow
api_obj_impl!(BearerToken,
              "access_token" => access_token: String,
              "token_type" => token_type: String,
              "expires_in" => expires_in: usize,
              "scope" => scope: String
);
