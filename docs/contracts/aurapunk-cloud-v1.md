# AuraPunk Cloud contract v1

This contract defines the compatibility boundary between the public desktop
core and the private AuraPunk Cloud control plane. It is separate from the
application release version.

## Discovery

The local backend endpoint `/api/app-mode` returns:

```json
{
  "mode": "cloud",
  "cloud": true,
  "cloud_url": "https://cloud.example.invalid",
  "cloud_contract_version": 1,
  "cloud_contract_path": "/api/cloud-contract"
}
```

The cloud endpoint `${cloud_url}/api/cloud-contract` returns public metadata:

```json
{
  "contract": "aurapunk-cloud",
  "version": 1,
  "service": "AuraPunk Cloud",
  "capabilities": {
    "desktop_auth_handoff": true,
    "account_identity": true,
    "entitlements": true,
    "memory_backend_routing": true,
    "usage_reporting": true,
    "preference_sync": true
  }
}
```

The discovery response is not an authentication mechanism. It must not contain
account IDs, session cookies, bearer tokens, provider keys, or tenant data.

## Compatibility rules

- Adding an optional capability does not require a new contract version.
- Removing a capability or changing the meaning of an existing field requires
  a new version.
- The desktop app must remain usable in local mode when discovery fails.
- A cloud deployment must reject unsupported contract versions explicitly and
  return an actionable compatibility error.
- Application release versions and contract versions are tracked separately.

## Version ownership

The public repository owns the contract definition and version. The private
service advertises the versions it implements and consumes the public core at a
tagged release. Changes to this file require compatibility tests in both
repositories.

