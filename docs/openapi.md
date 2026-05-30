# OpenAPI and TypeScript client

Rustwing generated apps are API-first by default. A new app exposes:

```txt
/openapi.json
/docs
/redoc
```

Run the API with `rustwing run`, then open `http://localhost:3000/docs` to inspect and test generated endpoints.

## Generated resources

`rustwing g resource` adds OpenAPI metadata automatically. Generated DTOs derive schemas, handlers include route annotations, and `api/src/openapi.rs` registers the new paths, schemas, and tag.

Scoped resources document route-driven scope params automatically:

```bash
rustwing g resource note   --tenant org_id   --scope ticket_id   --fields 'org_id:uuid:required'   --fields 'ticket_id:uuid:required'   --fields 'body:text:required'
```

This documents paths like:

```txt
/orgs/{org_id}/tickets/{ticket_id}/notes
/orgs/{org_id}/tickets/{ticket_id}/notes/{note_id}
```

Scope fields come from the URL and do not appear in create/update request bodies.

## Export the contract

```bash
rustwing g openapi
```

Default output:

```txt
openapi/openapi.json
```

Useful flags:

```bash
rustwing g openapi --output ./openapi.json
rustwing g openapi --stdout
rustwing g openapi --check
```

Use `--check` in CI:

```bash
rustwing g openapi --check
```

If the checked-in spec is stale, the command exits non-zero.

## Generate the TypeScript client

```bash
rustwing g client typescript
```

Default output:

```txt
frontend/generated/types.ts
frontend/generated/client.ts
frontend/generated/index.ts
```

Example usage:

```ts
import { createRustwingClient } from "./generated";

const api = createRustwingClient({
  baseUrl: "http://localhost:3000",
  getToken: () => localStorage.getItem("token"),
});

const ticket = await api.tickets.create({
  orgId: "00000000-0000-0000-0000-000000000000",
  body: {
    subject: "Billing issue",
    body: "I cannot access the billing page",
    status: "open",
  },
});
```

Generated methods follow resource names:

```ts
api.tickets.list({ orgId })
api.tickets.create({ orgId, body })
api.tickets.get({ orgId, ticketId })
api.tickets.update({ orgId, ticketId, body })
api.tickets.delete({ orgId, ticketId })
```

Nested resources include all scope params:

```ts
api.comments.list({ orgId, ticketId })
api.comments.create({ orgId, ticketId, body })
```

## Auth and errors

Authenticated generated endpoints reference the `bearerAuth` security scheme:

```txt
Authorization: Bearer <token>
```

Generated apps use a standard error envelope:

```json
{
  "error": {
    "code": "unauthorized",
    "message": "You must be logged in to access this resource"
  }
}
```

Resource endpoints document common errors such as validation errors, unauthorized access, not found, conflict, and internal server errors.

## Custom routes

Normal generated resources should not need manual OpenAPI work. For custom routes, add `utoipa::path` annotations to the handler and register the handler/schema in `api/src/openapi.rs`.
