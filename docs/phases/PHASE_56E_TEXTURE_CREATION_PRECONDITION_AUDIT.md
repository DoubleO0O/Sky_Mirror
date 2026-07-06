# Phase 56E - Texture Creation Precondition Audit

Phase 56E audits the gap between the Phase 56D SHM metadata validation harness
and any future texture creation path. It continues the SHM-first nested MVP
route, but it does not import a buffer, create a texture, call a renderer,
submit damage, send frame callback done, connect input, mutate core, enter DRM
/ GBM / dmabuf, enter EGL / GLES, enter WGPU, or add a Cargo dependency.

The phase produces pure-data texture precondition audit evidence only.

## Authorization

The user has authorized Phase 56E only. Phase 56E may consume the Phase 56D
validation harness and SHM metadata evidence reports. It may define a
`texture_precondition` report and expose it through runtime / bounded loop /
orchestrator summaries.

Phase 56E must not be treated as authorization for texture creation,
renderer calls, damage submit, frame callback done, input, or core mutation.

## Phase 56D To 56E Relationship

Phase 56D established a controlled validation harness for:

- no real WlBuffer path;
- non-SHM path;
- metadata unavailable path;
- metadata partially available path;
- metadata insufficient for texture precondition path;
- missing lifetime / cleanup ownership policy path;
- runtime evidence without import execution path.

Phase 56E consumes that validation result and asks whether the project has
enough evidence to allow texture creation preconditions. The answer remains no:
metadata validation coverage is useful, but the real resource owners and
policies required after metadata are still absent.

## Capability Truth

Phase 56E keeps real execution capability false:

- `buffer_import_attempted = false`
- `buffer_imported = false`
- `texture_created = false`
- `renderer_called = false`
- `damage_submitted = false`
- `frame_callback_done_sent = false`
- `input_support = false`
- `core_mutation_invoked = false`

`texture_precondition_allowed = false` and
`texture_precondition_blocked = true` are audit results. They do not represent
a real texture object or renderer integration.

## Texture Precondition Checklist

Each checklist item is pure data. No item means texture creation, renderer
call, damage submit, or frame callback done.

| Item | Meaning | Source evidence | Satisfied | Blocker | Texture creation? | Renderer call? | Recommended next step |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `metadata_validation_passed` | Phase 56D harness covered required paths | `validation_harness_report` | yes when invoked and all paths covered | `metadata_validation_missing` | no | no | Keep as prerequisite evidence |
| `metadata_sufficient_for_texture_precondition` | Metadata is enough to enter a future texture precondition gate | SHM metadata report plus policy audit | no | `metadata_insufficient` | no | no | Define stricter precondition gate later |
| `width_known` | Width metadata is available | SHM metadata evidence | no in current runtime report | `unknown_width` | no | no | Carry real WlBuffer metadata into adapter report |
| `height_known` | Height metadata is available | SHM metadata evidence | no in current runtime report | `unknown_height` | no | no | Carry real WlBuffer metadata into adapter report |
| `stride_known` | Stride metadata is available | SHM metadata evidence | no in current runtime report | `unknown_stride` | no | no | Audit stride compatibility rules |
| `format_known` | Format metadata is available | SHM metadata evidence | no in current runtime report | `unknown_format` | no | no | Audit accepted SHM formats |
| `buffer_kind_supported` | Buffer kind is supported for SHM-first MVP | SHM metadata taxonomy | no without concrete SHM evidence | `unsupported_buffer_kind` | no | no | Keep non-SHM blocked |
| `lifetime_policy_known` | Buffer lifetime ownership is defined | owner policy audit | no | `missing_lifetime_policy` | no | no | Define owner/lifetime boundary |
| `cleanup_policy_known` | Buffer cleanup ownership is defined | owner policy audit | no | `missing_cleanup_policy` | no | no | Define cleanup boundary |
| `renderer_backend_instance_available` | A real renderer backend instance exists | renderer backend owner audit | no | `missing_renderer_backend_instance` | no | no | Audit renderer backend instance ownership |
| `texture_import_route_available` | A concrete route exists to create/import a texture | texture route audit | no | `missing_texture_import_route` | no | no | Define texture import route boundary |
| `damage_to_texture_mapping_available` | Damage can be mapped to texture/render work | damage mapping audit | no | `missing_damage_to_texture_mapping` | no | no | Audit damage-to-texture mapping |
| `frame_callback_completion_policy_available` | Frame callback done policy is known | frame callback policy audit | no | `missing_frame_callback_completion_policy` | no | no | Define completion policy after render |
| `texture_precondition_allowed` | All required preconditions are safe enough to proceed | combined checklist | no | `runtime_evidence_without_texture_creation` | no | no | Wait for next authorized phase |

## Blocker Taxonomy

Phase 56E reports these blockers:

- `metadata_validation_missing`
- `metadata_insufficient`
- `unknown_width`
- `unknown_height`
- `unknown_stride`
- `unknown_format`
- `unsupported_buffer_kind`
- `missing_lifetime_policy`
- `missing_cleanup_policy`
- `missing_renderer_backend_instance`
- `missing_texture_import_route`
- `missing_damage_to_texture_mapping`
- `missing_frame_callback_completion_policy`
- `runtime_evidence_without_texture_creation`

The blockers are safe boundaries. They prevent shell, record, dry-run,
validation, or audit reports from being misreported as real buffer import or
real texture creation.

## Report Fields

Phase 56E adds narrow report fields such as:

- `texture_precondition_audit_available = true`
- `texture_precondition_allowed = false`
- `texture_precondition_blocked = true`
- `texture_precondition_blocker_reason`
- `metadata_sufficient_for_texture_precondition = false`
- `renderer_backend_instance_available = false`
- `texture_import_route_available = false`
- `damage_to_texture_mapping_available = false`
- `frame_callback_completion_policy_available = false`

It must not report:

- `texture_created = true`
- `renderer_called = true`
- `damage_submitted = true`
- `frame_callback_done_sent = true`

## Smithay Type Boundary

Real Smithay / Wayland / WlBuffer / SHM / BufferData / buffer / texture /
renderer types must remain inside `src/smithay_backend` Linux-only adapter or
glue layers.

Core must not depend on those types. Core remains limited to abstract concepts
such as `WindowId`, `Geometry`, `State`, `Layout`, `Action`, and `Command`.

## Phase 56F Direction

Phase 56F requires separate user authorization. Possible directions are:

1. texture creation blocker / no-op skeleton;
2. texture owner boundary;
3. renderer backend instance audit;
4. damage-to-texture mapping audit.

The default recommended Phase 56F is a texture creation blocker / no-op
skeleton. It should still not create a real texture unless the user explicitly
authorizes real texture creation.

## Stop Condition

Phase 56E stops after the texture precondition audit report is exposed through
runtime / bounded loop / orchestrator summaries and CI is green. Do not enter
Phase 56F without explicit authorization.
