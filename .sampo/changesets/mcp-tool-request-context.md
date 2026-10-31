---
cargo/posthog-rs: minor
---

MCP tool handlers now run inside a `RequestContext` carrying the call's `$session_id`, and its distinct ID when `identify` returns one. Spans and captures made in a tool join the same session as its `$mcp_tool_call` event. Anonymous calls set no distinct ID, so captures in the handler don't create a person.
