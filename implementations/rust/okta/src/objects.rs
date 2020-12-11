use serde::{Deserialize, Serialize};

/*
 * Used for extracting group user information from calling
 * https://dev-url.okta.com/api/v1/groups/{group_id}/users
 */
api_obj_impl!(UsersInGroup,
              "id" => id: String,
              "status" => status: String,
              "created" => created: String,
              "activated" => activated: String,
              "statusChanged" => status_changed: String,
              "lastLogin" => last_login: String,
              "lastUpdated" => last_updated: String,
              "passwordChanged" => password_changed: String,
              "type" => xtype: UsersInGroupType,
              "profile" => profile: UsersInGroupProfile,
              "credentials" => credentials: UsersInGroupCredentials,
              "_links" => links: UsersInGroupLinks
);

api_obj_impl!(UsersInGroupType,
              "id" => id: String
);

api_obj_impl!(UsersInGroupProfile,
              "firstName" => first_name: String,
              "lastName" => last_name: String,
              "mobilePhone" => mobile_phone: Option<String>,
              "secondEmail" => second_email: Option<String>,
              "login" => login: String,
              "email" => email: String
);

api_obj_impl!(UsersInGroupCredentials,
              "password" => password: Option<String>,
              "emails" => emails: Vec<UsersInGroupEmail>,
              "provider" => provider: UsersInGroupProvider
);

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
              "self" => inner: GroupUsers
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
              "users" => users: GroupUsers,
              "apps" => apps: GroupApps
);

api_obj_impl!(GroupLogo,
              "name" => name: String,
              "href" => href: String,
              "type" => xtype: String
);

api_obj_impl!(GroupUsers,
              "href" => href: String
);

api_obj_impl!(GroupApps,
              "href" => href: String
);
