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

typedef struct pp_project pp_project_t;
typedef struct pp_transaction pp_transaction_t;
typedef struct pp_resolution_set pp_resolution_set_t;
typedef struct pp_external_identifier_set pp_external_identifier_set_t;
typedef struct pp_object_ref_set pp_object_ref_set_t;
typedef struct pp_metadata_set pp_metadata_set_t;
typedef struct pp_metadata_value pp_metadata_value_t;
typedef struct pp_error pp_error_t;

typedef struct pp_uuid {
  uint8_t bytes[16];
} pp_uuid_t;

typedef uint32_t pp_object_kind_t;

#define PP_OBJECT_PROJECT UINT32_C(1)
#define PP_OBJECT_ASSET UINT32_C(2)
#define PP_OBJECT_REPRESENTATION UINT32_C(3)
#define PP_OBJECT_ACTIVITY UINT32_C(4)

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

typedef uint32_t pp_resolution_state_t;

#define PP_RESOLUTION_ONLINE_AT_KNOWN_LOCATION UINT32_C(1)
#define PP_RESOLUTION_RESOLVED_EXACT UINT32_C(2)
#define PP_RESOLUTION_RESOLVED_PROBABLE UINT32_C(3)
#define PP_RESOLUTION_MISSING UINT32_C(4)
#define PP_RESOLUTION_AMBIGUOUS UINT32_C(5)
#define PP_RESOLUTION_ERROR UINT32_C(6)

typedef uint32_t pp_evidence_kind_t;

#define PP_EVIDENCE_KNOWN_LOCATION_EXISTS UINT32_C(1)
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
 * absent. On success, *out_project is caller-owned and *out_error is NULL. On
 * failure, *out_project is NULL and a non-NULL *out_error is caller-owned.
 * out_error may itself be NULL when diagnostic text is not required. */
PP_API uint32_t pp_abi_version(void);
PP_API pp_error_code_t pp_project_create(const char *path,
                                         const char *display_name,
                                         pp_project_t **out_project,
                                         pp_error_t **out_error);
PP_API pp_error_code_t pp_project_open(const char *path,
                                       pp_project_t **out_project,
                                       pp_error_t **out_error);
PP_API pp_error_code_t pp_project_id(const pp_project_t *project,
                                     pp_uuid_t *out_id, pp_error_t **out_error);
PP_API pp_error_code_t pp_project_asset_exists(const pp_project_t *project,
                                               const pp_uuid_t *asset_id,
                                               uint8_t *out_exists,
                                               pp_error_t **out_error);
/* Result strings are borrowed until the owning result set is released. */
PP_API pp_error_code_t pp_project_external_identifiers(
    const pp_project_t *project, const pp_object_ref_t *target,
    pp_external_identifier_set_t **out_identifiers, pp_error_t **out_error);
PP_API pp_error_code_t pp_project_find_by_external_identifier(
    const pp_project_t *project, const char *scheme, const char *value,
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
PP_API pp_error_code_t pp_project_metadata(
    const pp_project_t *project, const pp_object_ref_t *target,
    pp_metadata_set_t **out_metadata, pp_error_t **out_error);
PP_API pp_error_code_t pp_project_find_metadata(
    const pp_project_t *project, const char *vocabulary, const char *property,
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
/* Resolution is read-only. Borrowed candidate URI and evidence-detail strings
 * remain valid until pp_resolution_set_release(). */
PP_API pp_error_code_t pp_project_resolve_asset(
    const pp_project_t *project, const pp_uuid_t *asset_id,
    pp_resolution_set_t **out_resolutions, pp_error_t **out_error);
PP_API uint64_t
pp_resolution_set_count(const pp_resolution_set_t *resolutions);
PP_API pp_error_code_t pp_resolution_set_get(
    const pp_resolution_set_t *resolutions, uint64_t resolution_index,
    pp_uuid_t *out_representation_id, pp_resolution_state_t *out_state,
    uint64_t *out_candidate_count, uint64_t *out_evidence_count,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_resolution_candidate_get(
    const pp_resolution_set_t *resolutions, uint64_t resolution_index,
    uint64_t candidate_index, const char **out_uri,
    uint16_t *out_confidence_basis_points, uint64_t *out_evidence_count,
    pp_error_t **out_error);
PP_API pp_error_code_t pp_resolution_evidence_get(
    const pp_resolution_set_t *resolutions, uint64_t resolution_index,
    uint64_t evidence_index, pp_evidence_kind_t *out_kind,
    const char **out_detail, pp_error_t **out_error);
PP_API pp_error_code_t pp_resolution_candidate_evidence_get(
    const pp_resolution_set_t *resolutions, uint64_t resolution_index,
    uint64_t candidate_index, uint64_t evidence_index,
    pp_evidence_kind_t *out_kind, const char **out_detail,
    pp_error_t **out_error);
PP_API void pp_resolution_set_release(pp_resolution_set_t *resolutions);
/* Only one transaction may be open for a project state. The transaction keeps
 * that state alive independently of the project handle. */
PP_API pp_error_code_t pp_project_begin_transaction(
    pp_project_t *project, pp_transaction_t **out_transaction,
    pp_error_t **out_error);
PP_API void pp_project_release(pp_project_t *project);

/* Mutations remain in memory until commit. Input strings are borrowed UTF-8
 * without embedded NUL. Nullable names/labels represent absent values. */
PP_API pp_error_code_t pp_transaction_import_media(
    pp_transaction_t *transaction, const char *path, const char *display_name,
    pp_uuid_t *out_asset_id, pp_error_t **out_error);
PP_API pp_error_code_t pp_transaction_add_media_root(
    pp_transaction_t *transaction, const char *path, const char *label,
    int32_t priority, pp_uuid_t *out_root_id, pp_error_t **out_error);
PP_API pp_error_code_t pp_transaction_confirm_location(
    pp_transaction_t *transaction, const pp_uuid_t *representation_id,
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
/* Text is required. Language may be NULL for plain text. Vocabulary and
 * property identifiers are exact UTF-8 strings and are not normalized. */
PP_API pp_error_code_t pp_transaction_add_metadata_text(
    pp_transaction_t *transaction, const pp_object_ref_t *target,
    const char *vocabulary, const char *property, const char *value,
    const char *language, pp_error_t **out_error);
PP_API pp_error_code_t pp_transaction_remove_metadata_property(
    pp_transaction_t *transaction, const pp_object_ref_t *target,
    const char *vocabulary, const char *property, pp_error_t **out_error);
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
