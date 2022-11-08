## ABAC policies API

Allows to get and set [ABAC](../../authorization/ABAC.md) policies.

Worker address: "abac_policies"

Implemented in `Ockam.Services.API.ABAC.PoliciesApi`

#### List policies
Method: GET
Path: ""
Request: ""
Response: `{* action_id => policy_rule}`

#### Show policy
Method: GET
Path: action_id
Request: ""
Response: policy_rule

#### Set policy
Method: PUT
Path: action_id
Request: policy_rule
Response: ""

Errors:
400 - cannot decode policy

#### Delete policy
Method: DELETE
Path: action_id
Request: ""
Response: ""

Where:
```
action_id = text ;; ":resource/:action"
policy_rule = text ;; ABAC s-expression rule
```

For more info on ABAC policies and rules see [ABAC](../../authorization/ABAC.md)
