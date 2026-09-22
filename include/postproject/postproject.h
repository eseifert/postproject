#ifndef POSTPROJECT_POSTPROJECT_H
#define POSTPROJECT_POSTPROJECT_H

#include <stdint.h>

#if defined(_WIN32) && defined(POSTPROJECT_SHARED)
#if defined(POSTPROJECT_BUILDING_LIBRARY)
#define PP_API __declspec(dllexport)
#else
#define PP_API __declspec(dllimport)
#endif
#elif defined(__GNUC__) && defined(POSTPROJECT_SHARED)
#define PP_API __attribute__((visibility("default")))
#else
#define PP_API
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef struct pp_production pp_production_t;
typedef struct pp_transaction pp_transaction_t;
typedef struct pp_asset_set pp_asset_set_t;
typedef struct pp_representation_set pp_representation_set_t;
typedef struct pp_resolution_set pp_resolution_set_t;
typedef struct pp_external_identifier_set pp_external_identifier_set_t;
typedef struct pp_object_ref_set pp_object_ref_set_t;
typedef struct pp_metadata_set pp_metadata_set_t;
typedef struct pp_metadata_value pp_metadata_value_t;
typedef struct pp_metadata_input pp_metadata_input_t;
typedef struct pp_activity_set pp_activity_set_t;
typedef struct pp_revision_set pp_revision_set_t;
typedef struct pp_revision_event_set pp_revision_event_set_t;
typedef struct pp_error pp_error_t;

/* Production handles may be moved between threads and called concurrently;
 * calls on one handle serialize internally. Transaction handles require
 * caller-side serialization. No handle may be released while another thread
 * is using it. Result-set and error handles are caller-serialized. */

typedef struct pp_uuid {
  uint8_t bytes[16];
} pp_uuid_t;

typedef uint32_t pp_object_kind_t;

#define PP_OBJECT_PRODUCTION UINT32_C(1)
#define PP_OBJECT_ASSET UINT32_C(2)
#define PP_OBJECT_REPRESENTATION UINT32_C(3)
#define PP_OBJECT_RESOURCE UINT32_C(4)
#define PP_OBJECT_ACTIVITY UINT32_C(5)

typedef uint32_t pp_representation_kind_t;

#define PP_REPRESENTATION_ORIGINAL UINT32_C(1)
#define PP_REPRESENTATION_PROXY UINT32_C(2)
#define PP_REPRESENTATION_OPTIMIZED UINT32_C(3)
#define PP_REPRESENTATION_DERIVED UINT32_C(4)

typedef uint32_t pp_content_structure_kind_t;

#define PP_CONTENT_SINGLE_RESOURCE UINT32_C(1)
#define PP_CONTENT_IMAGE_SEQUENCE UINT32_C(2)
#define PP_CONTENT_ORDERED_PARTS UINT32_C(3)
#define PP_CONTENT_PACKAGE UINT32_C(4)

typedef uint32_t pp_locator_availability_t;

#define PP_LOCATOR_UNKNOWN UINT32_C(1)
#define PP_LOCATOR_ONLINE UINT32_C(2)
#define PP_LOCATOR_OFFLINE UINT32_C(3)

typedef uint32_t pp_revision_event_kind_t;

#define PP_REVISION_ASSET_IMPORTED UINT32_C(1)
#define PP_REVISION_REPRESENTATION_ADDED UINT32_C(2)
#define PP_REVISION_RESOURCE_ADDED UINT32_C(3)
#define PP_REVISION_REPRESENTATION_RESOURCE_ADDED UINT32_C(4)
#define PP_REVISION_LOCATOR_ADDED UINT32_C(5)
#define PP_REVISION_MEDIA_ROOT_ADDED UINT32_C(6)
#define PP_REVISION_EXTERNAL_IDENTIFIER_ADDED UINT32_C(7)
#define PP_REVISION_EXTERNAL_IDENTIFIER_REMOVED UINT32_C(8)
#define PP_REVISION_METADATA_ADDED_OR_REPLACED UINT32_C(9)
#define PP_REVISION_METADATA_REMOVED UINT32_C(10)
#define PP_REVISION_ACTIVITY_CREATED UINT32_C(11)
#define PP_REVISION_ACTIVITY_INPUT_ADDED UINT32_C(12)
#define PP_REVISION_ACTIVITY_OUTPUT_ADDED UINT32_C(13)

typedef uint32_t pp_metadata_value_kind_t;

#define PP_METADATA_STRING UINT32_C(1)
#define PP_METADATA_LANG_STRING UINT32_C(2)
#define PP_METADATA_I64 UINT32_C(3)
#define PP_METADATA_U64 UINT32_C(4)
#define PP_METADATA_DECIMAL UINT32_C(5)
#define PP_METADATA_BOOL UINT32_C(6)
#define PP_METADATA_TIMESTAMP UINT32_C(7)
#define PP_METADATA_URI UINT32_C(8)
#define PP_METADATA_BYTES UINT32_C(9)
#define PP_METADATA_RATIONAL UINT32_C(10)
#define PP_METADATA_LIST UINT32_C(11)
#define PP_METADATA_STRUCT UINT32_C(12)
#define PP_METADATA_REFERENCE UINT32_C(13)

typedef struct pp_object_ref {
  pp_object_kind_t kind;
  pp_uuid_t id;
} pp_object_ref_t;

/* Fields not used by an event kind are zero or NULL. String pointers borrow
 * the owning pp_revision_event_set_t. */
typedef struct pp_revision_event {
  pp_revision_event_kind_t kind;
  uint32_t position;
  pp_uuid_t asset_id;
  pp_uuid_t representation_id;
  pp_uuid_t resource_id;
  pp_uuid_t locator_id;
  pp_uuid_t media_root_id;
  pp_uuid_t activity_id;
  pp_object_ref_t target;
  uint32_t structural_position;
  const char *identifier_scheme;
  const char *identifier_value;
  const char *identifier_qualifier;
  const char *vocabulary;
  const char *property;
  const char *activity_kind;
  const char *role;
} pp_revision_event_t;

typedef struct pp_activity_edge {
  pp_uuid_t representation_id;
  const char *role;
} pp_activity_edge_t;

typedef struct pp_file_resource_input {
  const char *path;
  const char *role;
  uint8_t required;
} pp_file_resource_input_t;

typedef uint32_t pp_error_code_t;

#define PP_OK UINT32_C(0)
#define PP_ERROR_INVALID_ARGUMENT UINT32_C(1)
#define PP_ERROR_NOT_FOUND UINT32_C(2)
#define PP_ERROR_ALREADY_EXISTS UINT32_C(3)
#define PP_ERROR_IO UINT32_C(4)
#define PP_ERROR_STORAGE UINT32_C(5)
#define PP_ERROR_MIGRATION UINT32_C(6)
#define PP_ERROR_CONFLICT UINT32_C(7)
#define PP_ERROR_AMBIGUOUS_RESOLUTION UINT32_C(8)
#define PP_ERROR_FINGERPRINT UINT32_C(9)
#define PP_ERROR_UNSUPPORTED UINT32_C(10)
#define PP_ERROR_INTERNAL UINT32_C(255)

typedef uint32_t pp_representation_availability_t;

#define PP_AVAILABILITY_ONLINE UINT32_C(1)
#define PP_AVAILABILITY_PARTIAL UINT32_C(2)
#define PP_AVAILABILITY_OFFLINE UINT32_C(3)
#define PP_AVAILABILITY_AMBIGUOUS UINT32_C(4)
#define PP_AVAILABILITY_ERROR UINT32_C(5)

typedef uint32_t pp_resource_resolution_state_t;

#define PP_RESOURCE_ONLINE_AT_KNOWN_LOCATOR UINT32_C(1)
#define PP_RESOURCE_RESOLVED_EXACT UINT32_C(2)
#define PP_RESOURCE_RESOLVED_PROBABLE UINT32_C(3)
#define PP_RESOURCE_OFFLINE UINT32_C(4)
#define PP_RESOURCE_AMBIGUOUS UINT32_C(5)
#define PP_RESOURCE_RESOLUTION_ERROR UINT32_C(6)

typedef uint32_t pp_availability_issue_kind_t;

#define PP_AVAILABILITY_ISSUE_OFFLINE_RESOURCE UINT32_C(1)
#define PP_AVAILABILITY_ISSUE_AMBIGUOUS_RESOURCE UINT32_C(2)
#define PP_AVAILABILITY_ISSUE_RESOURCE_ERROR UINT32_C(3)
#define PP_AVAILABILITY_ISSUE_MISSING_FRAMES UINT32_C(4)

typedef uint32_t pp_evidence_kind_t;

#define PP_EVIDENCE_KNOWN_LOCATOR_AVAILABLE UINT32_C(1)
#define PP_EVIDENCE_EXACT_FINGERPRINT_MATCH UINT32_C(2)
#define PP_EVIDENCE_FULL_HASH_MATCH UINT32_C(3)
#define PP_EVIDENCE_PARTIAL_FINGERPRINT_MATCH UINT32_C(4)
#define PP_EVIDENCE_FILE_SIZE_MATCH UINT32_C(5)
#define PP_EVIDENCE_FILE_NAME_MATCH UINT32_C(6)
#define PP_EVIDENCE_RELATIVE_PATH_SIMILARITY UINT32_C(7)
#define PP_EVIDENCE_MEDIA_ROOT_RELATION UINT32_C(8)
#define PP_EVIDENCE_CONFLICTING_CANDIDATE UINT32_C(9)
#define PP_EVIDENCE_DISCOVERY_ERROR UINT32_C(10)

/* Inputs are borrowed UTF-8 without embedded NUL. A NULL display name is
 * absent. On success, *out_production is caller-owned and *out_error is NULL. On
 * failure, *out_production is NULL and a non-NULL *out_error is caller-owned.
 * out_error may itself be NULL when diagnostic text is not required. */
PP_API uint32_t pp_abi_version(void);
/* Host bindings are pure value operations and perform no network access.
 * Inputs are borrowed. On success, *out_binding is caller-owned and must be
 * released exactly once with pp_host_binding_release(). */
PP_API pp_error_code_t pp_host_binding_format(
    const pp_uuid_t *production_id, const pp_object_ref_t *object,
    char **out_binding, pp_error_t **out_error);
/* binding is borrowed NUL-terminated UTF-8. Both value outputs are required
 * caller-owned storage and are cleared on failure. */
PP_API pp_error_code_t pp_host_binding_parse(
    const char *binding, pp_uuid_t *out_production_id,
    pp_object_ref_t *out_object, pp_error_t **out_error);
/* Accepts NULL. No pointer returned by another function may be passed here. */
PP_API void pp_host_binding_release(char *binding);
PP_API pp_error_code_t pp_production_create(const char *path,
                                         const char *display_name,
                                         pp_production_t **out_production,
                                         pp_error_t **out_error);
PP_API pp_error_code_t pp_production_open(const char *path,
                                       pp_production_t **out_production,
                                       pp_error_t **out_error);
PP_API pp_error_code_t pp_production_id(const pp_production_t *production,
                                     pp_uuid_t *out_id, pp_error_t **out_error);
PP_API pp_error_code_t pp_production_asset_exists(const pp_production_t *production,
                                               const pp_uuid_t *asset_id,
                                               uint8_t *out_exists,
                                               pp_error_t **out_error);
/* Asset strings borrow the owning result set. */
PP_API pp_error_code_t pp_production_assets(
    const pp_production_t *production, pp_asset_set_t **out_assets,
    pp_error_t **out_error);
PP_API uint64_t pp_asset_set_count(const pp_asset_set_t *assets);
PP_API pp_error_code_t pp_asset_set_get(
    const pp_asset_set_t *assets, uint64_t index, pp_uuid_t *out_id,
    int64_t *out_created_at_unix_micros, const char **out_display_name,
    const char **out_import_source, pp_error_t **out_error);
PP_API void pp_asset_set_release(pp_asset_set_t *assets);
/* Representation strings borrow the owning result set. Members are returned in
 * structural order. Single-resource and image-sequence members have no role. */
PP_API pp_error_code_t pp_production_representations(
    const pp_production_t *production, const pp_uuid_t *asset_id,
    pp_representation_set_t **out_representations, pp_error_t **out_error);
PP_API uint64_t pp_representation_set_count(
    const pp_representation_set_t *representations);
PP_API pp_error_code_t pp_representation_set_get(
    const pp_representation_set_t *representations, uint64_t index,
    pp_uuid_t *out_id, pp_uuid_t *out_asset_id,
    pp_representation_kind_t *out_kind,
    pp_content_structure_kind_t *out_structure_kind,
    uint64_t *out_member_count, uint64_t *out_resource_count,
    uint64_t *out_fingerprint_count, pp_error_t **out_error);
PP_API pp_error_code_t pp_representation_set_get_fingerprint(
    const pp_representation_set_t *representations,
    uint64_t representation_index, uint64_t fingerprint_index,
    const char **out_algorithm, uint16_t *out_version,
    const uint8_t **out_value, uint64_t *out_value_length,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_representation_set_get_member(
    const pp_representation_set_t *representations,
    uint64_t representation_index, uint64_t member_index,
    pp_uuid_t *out_resource_id, const char **out_role, uint8_t *out_required,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_representation_set_get_sequence(
    const pp_representation_set_t *representations,
    uint64_t representation_index, const char **out_prefix,
    const char **out_suffix, uint8_t *out_padding, int64_t *out_start,
    int64_t *out_end, uint32_t *out_step, uint32_t *out_rate_numerator,
    uint32_t *out_rate_denominator, uint64_t *out_missing_count,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_representation_set_get_sequence_missing_frame(
    const pp_representation_set_t *representations,
    uint64_t representation_index, uint64_t frame_index, int64_t *out_frame,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_representation_set_get_resource(
    const pp_representation_set_t *representations,
    uint64_t representation_index, uint64_t resource_index, pp_uuid_t *out_id,
    uint8_t *out_has_file_facts, uint64_t *out_file_size,
    uint8_t *out_has_modified_at, int64_t *out_modified_at_unix_micros,
    uint64_t *out_locator_count, uint64_t *out_fingerprint_count,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_representation_set_get_resource_fingerprint(
    const pp_representation_set_t *representations,
    uint64_t representation_index, uint64_t resource_index,
    uint64_t fingerprint_index, const char **out_algorithm,
    uint16_t *out_version, const uint8_t **out_value,
    uint64_t *out_value_length, pp_error_t **out_error);
PP_API pp_error_code_t pp_representation_set_get_locator(
    const pp_representation_set_t *representations,
    uint64_t representation_index, uint64_t resource_index,
    uint64_t locator_index, pp_uuid_t *out_id, const char **out_uri,
    pp_locator_availability_t *out_availability, uint8_t *out_has_last_seen,
    int64_t *out_last_seen_unix_micros, pp_error_t **out_error);
PP_API void pp_representation_set_release(
    pp_representation_set_t *representations);
/* Result strings are borrowed until the owning result set is released. */
PP_API pp_error_code_t pp_production_external_identifiers(
    const pp_production_t *production, const pp_object_ref_t *target,
    pp_external_identifier_set_t **out_identifiers, pp_error_t **out_error);
PP_API pp_error_code_t pp_production_find_by_external_identifier(
    const pp_production_t *production, const char *scheme, const char *value,
    pp_object_ref_set_t **out_objects, pp_error_t **out_error);
PP_API uint64_t pp_external_identifier_set_count(
    const pp_external_identifier_set_t *identifiers);
PP_API pp_error_code_t pp_external_identifier_set_get(
    const pp_external_identifier_set_t *identifiers, uint64_t index,
    const char **out_scheme, const char **out_value, const char **out_qualifier,
    pp_error_t **out_error);
PP_API void pp_external_identifier_set_release(
    pp_external_identifier_set_t *identifiers);
PP_API uint64_t
pp_object_ref_set_count(const pp_object_ref_set_t *objects);
PP_API pp_error_code_t pp_object_ref_set_get(
    const pp_object_ref_set_t *objects, uint64_t index,
    pp_object_ref_t *out_object, pp_error_t **out_error);
PP_API void pp_object_ref_set_release(pp_object_ref_set_t *objects);
/* Metadata result sets own every returned string and recursively typed value.
 * All pointers borrowed from a set become invalid when that set is released. */
PP_API pp_error_code_t pp_production_metadata(
    const pp_production_t *production, const pp_object_ref_t *target,
    pp_metadata_set_t **out_metadata, pp_error_t **out_error);
PP_API pp_error_code_t pp_production_find_metadata(
    const pp_production_t *production, const char *vocabulary, const char *property,
    pp_metadata_set_t **out_metadata, pp_error_t **out_error);
PP_API uint64_t pp_metadata_set_count(const pp_metadata_set_t *metadata);
PP_API pp_error_code_t pp_metadata_set_get(
    const pp_metadata_set_t *metadata, uint64_t index,
    pp_object_ref_t *out_target, const char **out_vocabulary,
    const char **out_property, const pp_metadata_value_t **out_value,
    pp_error_t **out_error);
PP_API void pp_metadata_set_release(pp_metadata_set_t *metadata);
PP_API pp_metadata_value_kind_t
pp_metadata_value_kind(const pp_metadata_value_t *value);
PP_API pp_error_code_t pp_metadata_value_get_string(
    const pp_metadata_value_t *value, const char **out_text,
    const char **out_language, pp_error_t **out_error);
PP_API pp_error_code_t pp_metadata_value_get_i64(
    const pp_metadata_value_t *value, int64_t *out_value,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_metadata_value_get_u64(
    const pp_metadata_value_t *value, uint64_t *out_value,
    pp_error_t **out_error);
/* Decimal coefficient is exact base-ten text borrowed from the result set. */
PP_API pp_error_code_t pp_metadata_value_get_decimal(
    const pp_metadata_value_t *value, const char **out_coefficient,
    uint32_t *out_scale, pp_error_t **out_error);
PP_API pp_error_code_t pp_metadata_value_get_bool(
    const pp_metadata_value_t *value, uint8_t *out_value,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_metadata_value_get_timestamp(
    const pp_metadata_value_t *value, int64_t *out_unix_micros,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_metadata_value_get_uri(
    const pp_metadata_value_t *value, const char **out_uri,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_metadata_value_get_bytes(
    const pp_metadata_value_t *value, const uint8_t **out_bytes,
    uint64_t *out_length, pp_error_t **out_error);
PP_API pp_error_code_t pp_metadata_value_get_rational(
    const pp_metadata_value_t *value, int64_t *out_numerator,
    uint64_t *out_denominator, pp_error_t **out_error);
PP_API uint64_t
pp_metadata_value_list_count(const pp_metadata_value_t *value);
PP_API pp_error_code_t pp_metadata_value_list_get(
    const pp_metadata_value_t *value, uint64_t index,
    const pp_metadata_value_t **out_item, pp_error_t **out_error);
PP_API uint64_t
pp_metadata_value_struct_count(const pp_metadata_value_t *value);
PP_API pp_error_code_t pp_metadata_value_struct_get(
    const pp_metadata_value_t *value, uint64_t index, const char **out_name,
    const pp_metadata_value_t **out_field_value, pp_error_t **out_error);
PP_API pp_error_code_t pp_metadata_value_get_reference(
    const pp_metadata_value_t *value, pp_object_ref_t *out_reference,
    pp_error_t **out_error);
/* Metadata inputs are owned immutable values. Collection constructors borrow
 * children only for the call and copy them. */
PP_API pp_error_code_t pp_metadata_input_create_string(
    const char *text, const char *language, pp_metadata_input_t **out_input,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_metadata_input_create_i64(
    int64_t value, pp_metadata_input_t **out_input, pp_error_t **out_error);
PP_API pp_error_code_t pp_metadata_input_create_u64(
    uint64_t value, pp_metadata_input_t **out_input, pp_error_t **out_error);
PP_API pp_error_code_t pp_metadata_input_create_decimal(
    const char *coefficient, uint32_t scale, pp_metadata_input_t **out_input,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_metadata_input_create_bool(
    uint8_t value, pp_metadata_input_t **out_input, pp_error_t **out_error);
PP_API pp_error_code_t pp_metadata_input_create_timestamp(
    int64_t unix_micros, pp_metadata_input_t **out_input,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_metadata_input_create_uri(
    const char *uri, pp_metadata_input_t **out_input, pp_error_t **out_error);
PP_API pp_error_code_t pp_metadata_input_create_bytes(
    const uint8_t *bytes, uint64_t length, pp_metadata_input_t **out_input,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_metadata_input_create_rational(
    int64_t numerator, uint64_t denominator, pp_metadata_input_t **out_input,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_metadata_input_create_reference(
    const pp_object_ref_t *target, pp_metadata_input_t **out_input,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_metadata_input_create_list(
    const pp_metadata_input_t **items, uint64_t count,
    pp_metadata_input_t **out_input, pp_error_t **out_error);
PP_API pp_error_code_t pp_metadata_input_create_struct(
    const char **names, const pp_metadata_input_t **values, uint64_t count,
    pp_metadata_input_t **out_input, pp_error_t **out_error);
PP_API void pp_metadata_input_release(pp_metadata_input_t *input);
/* Activity strings are borrowed until pp_activity_set_release(). Optional
 * timestamps use explicit presence flags and zero values when absent. */
PP_API pp_error_code_t pp_production_activities(
    const pp_production_t *production, pp_activity_set_t **out_activities,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_production_activities_producing(
    const pp_production_t *production, const pp_uuid_t *representation_id,
    pp_activity_set_t **out_activities, pp_error_t **out_error);
PP_API pp_error_code_t pp_production_activities_consuming(
    const pp_production_t *production, const pp_uuid_t *representation_id,
    pp_activity_set_t **out_activities, pp_error_t **out_error);
PP_API pp_error_code_t pp_production_provenance_ancestors(
    const pp_production_t *production, const pp_uuid_t *representation_id,
    pp_object_ref_set_t **out_representations, pp_error_t **out_error);
PP_API pp_error_code_t pp_production_provenance_descendants(
    const pp_production_t *production, const pp_uuid_t *representation_id,
    pp_object_ref_set_t **out_representations, pp_error_t **out_error);
PP_API uint64_t pp_activity_set_count(const pp_activity_set_t *activities);
PP_API pp_error_code_t pp_activity_set_get(
    const pp_activity_set_t *activities, uint64_t index, pp_uuid_t *out_id,
    const char **out_kind, uint8_t *out_has_started_at,
    int64_t *out_started_at_unix_micros, uint8_t *out_has_finished_at,
    int64_t *out_finished_at_unix_micros, uint64_t *out_input_count,
    uint64_t *out_output_count, pp_error_t **out_error);
PP_API pp_error_code_t pp_activity_set_get_tool(
    const pp_activity_set_t *activities, uint64_t index, const char **out_name,
    const char **out_version, const char **out_uri, pp_error_t **out_error);
PP_API pp_error_code_t pp_activity_set_get_agent(
    const pp_activity_set_t *activities, uint64_t index, const char **out_name,
    const char **out_identifier_scheme, const char **out_identifier_value,
    const char **out_identifier_qualifier, pp_error_t **out_error);
PP_API pp_error_code_t pp_activity_set_get_input(
    const pp_activity_set_t *activities, uint64_t activity_index,
    uint64_t input_index, pp_uuid_t *out_representation_id,
    const char **out_role, pp_error_t **out_error);
PP_API pp_error_code_t pp_activity_set_get_output(
    const pp_activity_set_t *activities, uint64_t activity_index,
    uint64_t output_index, pp_uuid_t *out_representation_id,
    const char **out_role, pp_error_t **out_error);
PP_API void pp_activity_set_release(pp_activity_set_t *activities);
/* Revision strings are borrowed until pp_revision_set_release(). Latest
 * returns a set containing zero or one revision. */
PP_API pp_error_code_t pp_production_latest_revision(
    const pp_production_t *production, pp_revision_set_t **out_revisions,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_production_changes_since(
    const pp_production_t *production, uint64_t sequence, uint32_t limit,
    pp_revision_set_t **out_revisions, pp_error_t **out_error);
PP_API uint64_t pp_revision_set_count(const pp_revision_set_t *revisions);
PP_API pp_error_code_t pp_revision_set_get(
    const pp_revision_set_t *revisions, uint64_t index, pp_uuid_t *out_id,
    uint64_t *out_sequence, pp_uuid_t *out_transaction_id,
    int64_t *out_committed_at_unix_micros, const char **out_origin_name,
    const char **out_origin_version, const char **out_origin_uri,
    const char **out_message, pp_error_t **out_error);
PP_API void pp_revision_set_release(pp_revision_set_t *revisions);
PP_API pp_error_code_t pp_production_revision_events(
    const pp_production_t *production, const pp_uuid_t *revision_id,
    pp_revision_event_set_t **out_events, pp_error_t **out_error);
PP_API uint64_t
pp_revision_event_set_count(const pp_revision_event_set_t *events);
PP_API pp_error_code_t pp_revision_event_set_get(
    const pp_revision_event_set_t *events, uint64_t index,
    pp_revision_event_t *out_event, pp_error_t **out_error);
PP_API void pp_revision_event_set_release(pp_revision_event_set_t *events);
/* Resolution is read-only. Borrowed candidate URI and evidence-detail strings
 * remain valid until pp_resolution_set_release(). */
PP_API pp_error_code_t pp_production_resolve_asset(
    const pp_production_t *production, const pp_uuid_t *asset_id,
    pp_resolution_set_t **out_resolutions, pp_error_t **out_error);
PP_API uint64_t pp_resolution_set_representation_count(
    const pp_resolution_set_t *resolutions);
PP_API pp_error_code_t pp_resolution_set_get_representation(
    const pp_resolution_set_t *resolutions, uint64_t representation_index,
    pp_uuid_t *out_representation_id,
    pp_representation_availability_t *out_availability,
    uint64_t *out_resource_count, uint64_t *out_issue_count,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_resolution_set_get_resource(
    const pp_resolution_set_t *resolutions, uint64_t representation_index,
    uint64_t resource_index, pp_uuid_t *out_resource_id,
    pp_resource_resolution_state_t *out_state,
    uint64_t *out_candidate_count, uint64_t *out_evidence_count,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_resolution_set_get_issue(
    const pp_resolution_set_t *resolutions, uint64_t representation_index,
    uint64_t issue_index, pp_uuid_t *out_resource_id, uint8_t *out_required,
    pp_availability_issue_kind_t *out_kind, uint64_t *out_frame_count,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_resolution_set_get_issue_frame(
    const pp_resolution_set_t *resolutions, uint64_t representation_index,
    uint64_t issue_index, uint64_t frame_index, int64_t *out_frame,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_resolution_set_get_candidate(
    const pp_resolution_set_t *resolutions, uint64_t representation_index,
    uint64_t resource_index, uint64_t candidate_index, const char **out_uri,
    uint16_t *out_confidence_basis_points, uint64_t *out_evidence_count,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_resolution_set_get_resource_evidence(
    const pp_resolution_set_t *resolutions, uint64_t representation_index,
    uint64_t resource_index, uint64_t evidence_index,
    pp_evidence_kind_t *out_kind, const char **out_detail,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_resolution_set_get_candidate_evidence(
    const pp_resolution_set_t *resolutions, uint64_t representation_index,
    uint64_t resource_index, uint64_t candidate_index,
    uint64_t evidence_index, pp_evidence_kind_t *out_kind,
    const char **out_detail, pp_error_t **out_error);
PP_API void pp_resolution_set_release(pp_resolution_set_t *resolutions);
/* Only one transaction may be open for a production state. The transaction keeps
 * that state alive independently of the production handle. Staging does not
 * block production reads; commit serializes with calls on the same production
 * state. Open the production again to avoid that per-handle serialization. */
PP_API pp_error_code_t pp_production_begin_transaction(
    pp_production_t *production, pp_transaction_t **out_transaction,
    pp_error_t **out_error);
PP_API void pp_production_release(pp_production_t *production);

/* Mutations remain in memory until commit. Transaction calls require caller-side
 * serialization. Input strings are borrowed UTF-8 without embedded NUL.
 * Nullable names/labels represent absent values. */
PP_API pp_error_code_t pp_transaction_set_revision_context(
    pp_transaction_t *transaction, const char *origin_name,
    const char *origin_version, const char *origin_uri, const char *message,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_transaction_import_media(
    pp_transaction_t *transaction, const char *path, const char *display_name,
    pp_uuid_t *out_asset_id, pp_error_t **out_error);
PP_API pp_error_code_t pp_transaction_add_single_file_representation(
    pp_transaction_t *transaction, const pp_uuid_t *asset_id,
    pp_representation_kind_t kind, const char *path,
    pp_uuid_t *out_representation_id, pp_error_t **out_error);
/* The missing-frame array is borrowed and may be NULL only when its count is
 * zero. The directory and pattern strings are required borrowed UTF-8. */
PP_API pp_error_code_t pp_transaction_add_image_sequence_representation(
    pp_transaction_t *transaction, const pp_uuid_t *asset_id,
    pp_representation_kind_t kind, const char *directory, const char *prefix,
    const char *suffix, uint8_t padding, int64_t start, int64_t end,
    uint32_t step, uint32_t rate_numerator, uint32_t rate_denominator,
    const int64_t *missing_frames, uint64_t missing_frame_count,
    pp_uuid_t *out_representation_id, pp_error_t **out_error);
/* Member arrays and their strings are borrowed only for the call. Ordered
 * parts must all be required; packages must contain a required member. */
PP_API pp_error_code_t pp_transaction_add_ordered_parts_representation(
    pp_transaction_t *transaction, const pp_uuid_t *asset_id,
    pp_representation_kind_t kind, const pp_file_resource_input_t *members,
    uint64_t member_count, pp_uuid_t *out_representation_id,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_transaction_add_package_representation(
    pp_transaction_t *transaction, const pp_uuid_t *asset_id,
    pp_representation_kind_t kind, const pp_file_resource_input_t *members,
    uint64_t member_count, pp_uuid_t *out_representation_id,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_transaction_add_media_root(
    pp_transaction_t *transaction, const char *path, const char *label,
    int32_t priority, pp_uuid_t *out_root_id, pp_error_t **out_error);
PP_API pp_error_code_t pp_transaction_confirm_locator(
    pp_transaction_t *transaction, const pp_uuid_t *resource_id,
    const char *uri, pp_error_t **out_error);
/* Scheme and value are required borrowed UTF-8 without embedded NUL. Qualifier
 * may be NULL. The complete mutation is validated and persisted at commit. */
PP_API pp_error_code_t pp_transaction_add_external_identifier(
    pp_transaction_t *transaction, const pp_object_ref_t *target,
    const char *scheme, const char *value, const char *qualifier,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_transaction_remove_external_identifier(
    pp_transaction_t *transaction, const pp_object_ref_t *target,
    const char *scheme, const char *value, const char *qualifier,
    pp_error_t **out_error);
/* Vocabulary and property identifiers are exact UTF-8 strings and are not
 * normalized. The input remains owned by the caller. */
PP_API pp_error_code_t pp_transaction_add_metadata_value(
    pp_transaction_t *transaction, const pp_object_ref_t *target,
    const char *vocabulary, const char *property,
    const pp_metadata_input_t *input, pp_error_t **out_error);
PP_API pp_error_code_t pp_transaction_remove_metadata_property(
    pp_transaction_t *transaction, const pp_object_ref_t *target,
    const char *vocabulary, const char *property, pp_error_t **out_error);
/* Arrays and strings are borrowed only for this call. A NULL timestamp pointer
 * means absent. Tool and agent fields are independently optional subject to
 * the documented domain invariants. */
PP_API pp_error_code_t pp_transaction_create_activity(
    pp_transaction_t *transaction, const char *kind,
    const pp_activity_edge_t *inputs, uint64_t input_count,
    const pp_activity_edge_t *outputs, uint64_t output_count,
    const int64_t *started_at_unix_micros,
    const int64_t *finished_at_unix_micros, const char *tool_name,
    const char *tool_version, const char *tool_uri, const char *agent_name,
    const char *agent_identifier_scheme, const char *agent_identifier_value,
    const char *agent_identifier_qualifier, pp_uuid_t *out_activity_id,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_transaction_commit(pp_transaction_t *transaction,
                                             pp_error_t **out_error);
PP_API pp_error_code_t pp_transaction_rollback(pp_transaction_t *transaction,
                                               pp_error_t **out_error);
/* Releasing an open transaction discards all staged work. Null is a no-op. */
PP_API void pp_transaction_release(pp_transaction_t *transaction);

PP_API pp_error_code_t pp_error_code(const pp_error_t *error);
/* The returned string is borrowed and valid until pp_error_release(error). */
PP_API const char *pp_error_message(const pp_error_t *error);
PP_API void pp_error_release(pp_error_t *error);

#ifdef __cplusplus
}
#endif

#endif
