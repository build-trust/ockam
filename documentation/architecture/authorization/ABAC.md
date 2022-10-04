## Attribute based authorization checks

ABAC is an authorization decision technique which allows us to authorize requests
using sets of attributes derived from request and resource it's trying to access.

### Attribute sets

Attributes are divided into three categories:

- Subject attributes, belonging to the caller
- Action attributes, describing the action
- Resource attributes, belonging to resource being accessed

For example subject can be user in the system, and it has an attribute "email"
Action can be a type of request, for example "Update"
Action can also have additional attributes, like "field"
And resource is a project we're trying to access, which can have an attribute "owners"

Then we can define our access policy to only let project owners update "space" field as:

To perform an action "update", if action attribute "field" is "services", the subject attribute "email" should be a member of a list, which is a resource attribute "owners".

### Policy enforcement and decision points

Policy Enforcement Point (Integration) - a place in the code, which using the current code context
can create and ABAC request.

Policy Decision Point (Policy Check) - system which can decide whether request is authorized or not.

Policy Information Point (Policy Storage) - a storage which keeps information about policies.

PDP and PIP can be abstracted and implemented as a separate system and even delegated
to remote nodes.

PEP is an integration point to convert existing request context into ABAC terms.

For example in message flow authorization PEP is an `is_authorized` function.
It takes a message and a worker state and creates an ABAC request which contains
subject, action and resource attributes.

### ABAC requests and policy rules

ABAC request is a set of attributes derived from the current request and context by PEP.

ABAC policy is a set of rules applied to the attributes.

In order to make request-policy association easier, both contain ActionID, which
describes a resource and an action being used.
This allows for example to use it as a key in the database.

For example ActionID for example above can be written as `Project/Update`

#### Policy rules

Policy rules for Ockam ABAC are defined as s-expressions.
Each rule resolves into boolean value when "true" means "allow".

##### Bool rule:

- `true`
- `false`

Mostly useful in logical rules, but can be used as allow-all or deny-all policies

##### Comparison rules:

- equality `(= subject.foo "bar")` `(!= subject.foo "bar")`
- numeric comparison `(> subject.foo 1)`, `(< subject.foo 3)`

Supports operators `=` `!=` `>` and `<`
First argument MUST be an Attribute
Second argument can be an Attribute or a Value

##### Inclustion rule:

Matches that an attribute belongs to a list.

- `(member? subject.key [1 2 3])`
- `(member? subject.key resource.keys)`
- `(member? "foo" action.keys)`

First argument can be an Attribute or a Value
Second argument can be an Attribute or a List
If second argument is an Attribute, it MUST resolve to a List

##### Logical rules:

Combine other rules

- `(not (member? "foo" action.keys))`
- `(and (= subject.key, "foo") (> subject.things 2))`
- `(or (= subject.key, "foo") (> subject.things 2) (= action.thing "smth"))`
- `(if (= subject.role "member") (member? subject.name resource.members) (= action.special true))`

`not` - unary operator
`if` - ternary operator with condition rule, true rule and false rule
  if condition rule matches - true rule is checkes
  if condition rule doesn't match - false rule is checked
`and|or` - operators combining multiple rules (can be 2 and more)

##### Attributes

- `subject.key`
- `action.other_key`
- `resource.something`

Attribute consists of type and name.
Type can be subject, action or resource
Name can contain lowercase ASCII letters, numbers and `_`

##### Values

- `"foo"`
- `1`
- `0.35`
- `true`

Values can be strings, numbers or booleans.

##### Lists

- `[1 2 3]`
- `["foo" "bar"]`

List consists of multiple values.

**Attributes, values and lists can be arguments in rules, but cannot be rules,
because they don't resolve to boolean**

### Putting all together

Now when we have tools to define policies and create ABAC requests we can describe
the example in ABAC terms.

First we'll need a PEP to convert existing request to ABAC attributes:

When we get a request:
address: "projects_api"
method: PUT
path: "projects/foo/services"
body: ...

Coming from a user with email: "foo@bar"

We can create an AttributeID: "Project/Update" - is derived from method
We can create subject attributes: `{email: "foo@bar"}`
We can create action attributes: `{field: "services"}` - derived from path
We can fetch the resource project with id: "foo"
and if it has a DB field "owners", create the resource attributes: `{owners: ["foo@bar", "baz@bar"]}`

This will result in:
```
ABACRequest{
action_id: "Project/Update",
subject_attributes: {email: "foo@bar"},
action_attributes: {field: "services"},
resource_attributes: {owners: ["foo@bar", "baz@bar"]}
}
```

Then we can fetch all policies from the policy storage using ActionID.

We can then describe the policy as:
```
{
  action_id: "Project/Update",
  rules: (and (= action.field "services") (member? subject.email resource.owners))
}
```

### ABAC and message flow authorization

Ockam workers allow to define `is_authorized` function to check if the message received
should be processed by the worker.

ABAC allows us to generalize authorization and make it work with (potentially dynamic) policies.

In that case resources are workers, actions are messages and subject is a sender of the message.

Resource attributes would be:
- worker address
- worker custom attributes configured on creation or some other point

Action attributes would be:
- Specific address the message was sent to (for multi-address workers)
- Address the message was sent from
- Local metadata fields of the message

Subject attributes would be:
- Message identity_id if message is coming from a secure channel
- Identity credentials, if this identity presented credentials

Because the action which is being authorized is handling of the message,
ActionID for message flow authorization would look like `<address>/handle_message`.
