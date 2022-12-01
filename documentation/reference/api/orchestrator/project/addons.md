## Project Addons API

Control addons configuration for a project.

Worker address: "projects"

Authorization:
- Requires connection via secure channel
- Identity needs to be enrolled to the Orchestrator Controller via [Auth0](./auth0.md)


#### List addons
Method: GET \
Path: "/v0/:project_id/addons" \
Request: "" \
Response: AddonItem

Errors:
- 404 - project not found
- 401 - current identity does not have permission to list addons from the project

#### Enable addon
Method: PUT \
Path: "/v0/:project_id/addons/:addon_id" \
Request: SpecificAddonConfig \
Response: ""

Errors:
- 404 - project not found
- 401 - current identity does not have permission to enable addons from the project
- 400 - addon config is invalid
- 400 - addon_id is invalid/unknown

#### Disable addon
Method: DELETE \
Path: "/v0/:project_id/addons/:addon_id" \
Request: "" \
Response: ""

Errors:
- 404 - project not found
- 401 - current identity does not have permission to disable addons from the project
- 400 - addon_id is invalid/unknown


Where:
```
AddonItem: {
  id: text,
  description: text,
  enabled: boolean
}
SpecificAddonConfig ;; Data structure specific for each addon
                    ;; Currently we only have Okta addon
OktaAddonConfig: {
  tenant_base_url: text,
  certificate: text,
  client_id: text,
  attributes: [text]
}
```
