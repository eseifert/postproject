#include <postproject/postproject.h>

#include <stdint.h>
#include <stdio.h>
#include <string.h>

static int uuid_is_zero(const pp_uuid_t *id) {
  static const uint8_t zero[16] = {0};
  return memcmp(id->bytes, zero, sizeof(zero)) == 0;
}

int main(int argc, char **argv) {
  pp_production_t *production = NULL;
  pp_transaction_t *transaction = NULL;
  pp_error_t *error = NULL;
  pp_uuid_t id = {{0}};
  pp_uuid_t asset_id = {{0}};
  pp_uuid_t rolled_back_asset_id = {{0}};
  pp_uuid_t root_id = {{0}};
  pp_uuid_t representation_id = {{0}};
  pp_uuid_t resource_id = {{0}};
  pp_uuid_t revision_id = {{0}};
  pp_uuid_t revision_transaction_id = {{0}};
  pp_revision_event_t revision_event = {0};
  char media_path[4096];
  char moved_media_path[4096];

  if (argc != 3) {
    return 64;
  }
  (void)remove(argv[1]);
  if (pp_abi_version() != UINT32_C(10)) {
    return 1;
  }
  pp_error_code_t status =
      pp_production_create(argv[1], "C smoke test", &production, &error);
  if (status != PP_OK) {
    fprintf(stderr, "create failed (%u): %s\n", status,
            error != NULL ? pp_error_message(error) : "no details");
    pp_error_release(error);
    return 2;
  }
  if (pp_production_id(production, &id, &error) != PP_OK || uuid_is_zero(&id)) {
    pp_production_release(production);
    pp_error_release(error);
    return 3;
  }
  int media_path_length =
      snprintf(media_path, sizeof(media_path), "%s.media", argv[1]);
  if (media_path_length < 0 ||
      (size_t)media_path_length >= sizeof(media_path)) {
    pp_production_release(production);
    return 6;
  }
  FILE *media = fopen(media_path, "wb");
  if (media == NULL) {
    pp_production_release(production);
    return 7;
  }
  size_t written = fwrite("C ABI media", 1, 11, media);
  int close_status = fclose(media);
  if (written != 11 || close_status != 0) {
    pp_production_release(production);
    return 7;
  }
  status = pp_production_begin_transaction(production, &transaction, &error);
  if (status != PP_OK || transaction == NULL) {
    pp_production_release(production);
    pp_error_release(error);
    return 8;
  }
  status = pp_transaction_import_media(transaction, media_path, NULL,
                                       &rolled_back_asset_id, &error);
  if (status != PP_OK || uuid_is_zero(&rolled_back_asset_id) ||
      pp_transaction_rollback(transaction, &error) != PP_OK) {
    pp_transaction_release(transaction);
    pp_production_release(production);
    pp_error_release(error);
    return 9;
  }
  pp_transaction_release(transaction);
  transaction = NULL;
  uint8_t asset_exists = 1;
  status = pp_production_asset_exists(production, &rolled_back_asset_id,
                                   &asset_exists, &error);
  if (status != PP_OK || asset_exists != UINT8_C(0)) {
    pp_production_release(production);
    pp_error_release(error);
    return 10;
  }

  status = pp_production_begin_transaction(production, &transaction, &error);
  if (status != PP_OK || transaction == NULL) {
    pp_production_release(production);
    pp_error_release(error);
    return 11;
  }
  status = pp_transaction_set_revision_context(
      transaction, "C smoke", "1.0", NULL, "Import fixture", &error);
  if (status != PP_OK) {
    pp_transaction_release(transaction);
    pp_production_release(production);
    pp_error_release(error);
    return 42;
  }
  status = pp_transaction_import_media(transaction, media_path, "C asset",
                                       &asset_id, &error);
  if (status != PP_OK || uuid_is_zero(&asset_id)) {
    pp_transaction_release(transaction);
    pp_production_release(production);
    pp_error_release(error);
    return 12;
  }
  pp_object_ref_t asset_ref = {PP_OBJECT_ASSET, asset_id};
  char *host_binding = NULL;
  pp_uuid_t bound_production_id = {{0}};
  pp_object_ref_t bound_object = {0};
  status = pp_host_binding_format(&id, &asset_ref, &host_binding, &error);
  if (status != PP_OK || host_binding == NULL ||
      strncmp(host_binding, "https://postproject.org/ref/v1/",
              sizeof("https://postproject.org/ref/v1/") - 1) != 0 ||
      pp_host_binding_parse(host_binding, &bound_production_id, &bound_object,
                            &error) != PP_OK ||
      memcmp(bound_production_id.bytes, id.bytes, sizeof(id.bytes)) != 0 ||
      bound_object.kind != PP_OBJECT_ASSET ||
      memcmp(bound_object.id.bytes, asset_id.bytes, sizeof(asset_id.bytes)) !=
          0) {
    pp_host_binding_release(host_binding);
    pp_transaction_release(transaction);
    pp_production_release(production);
    pp_error_release(error);
    return 65;
  }
  pp_host_binding_release(host_binding);
  status = pp_transaction_add_external_identifier(
      transaction, &asset_ref, "com.example.asset", "asset-42", "primary",
      &error);
  if (status != PP_OK) {
    pp_transaction_release(transaction);
    pp_production_release(production);
    pp_error_release(error);
    return 23;
  }
  status = pp_transaction_add_metadata_text(
      transaction, &asset_ref, "com.example.metadata", "title", "C title",
      "en-US", &error);
  if (status != PP_OK) {
    pp_transaction_release(transaction);
    pp_production_release(production);
    pp_error_release(error);
    return 26;
  }
  status = pp_transaction_add_media_root(transaction, argv[2], "fixture root",
                                         0, &root_id, &error);
  if (status != PP_OK || uuid_is_zero(&root_id)) {
    pp_transaction_release(transaction);
    pp_production_release(production);
    pp_error_release(error);
    return 13;
  }
  status = pp_transaction_commit(transaction, &error);
  if (status != PP_OK) {
    pp_transaction_release(transaction);
    pp_production_release(production);
    pp_error_release(error);
    return 14;
  }
  pp_transaction_release(transaction);
  pp_production_release(production);
  production = NULL;

  status = pp_production_open(argv[1], &production, &error);
  if (status != PP_OK) {
    fprintf(stderr, "open failed (%u): %s\n", status,
            error != NULL ? pp_error_message(error) : "no details");
    pp_error_release(error);
    return 4;
  }
  pp_revision_set_t *revisions = NULL;
  uint64_t revision_sequence = 0;
  int64_t revision_committed_at = 0;
  const char *revision_origin_name = NULL;
  const char *revision_origin_version = NULL;
  const char *revision_origin_uri = NULL;
  const char *revision_message = NULL;
  status = pp_production_latest_revision(production, &revisions, &error);
  if (status != PP_OK || revisions == NULL ||
      pp_revision_set_count(revisions) != UINT64_C(1) ||
      pp_revision_set_get(
          revisions, 0, &revision_id, &revision_sequence,
          &revision_transaction_id, &revision_committed_at,
          &revision_origin_name, &revision_origin_version,
          &revision_origin_uri, &revision_message, &error) != PP_OK ||
      uuid_is_zero(&revision_id) || uuid_is_zero(&revision_transaction_id) ||
      revision_sequence != UINT64_C(1) || revision_committed_at == 0 ||
      revision_origin_name == NULL || revision_origin_version == NULL ||
      revision_message == NULL ||
      strcmp(revision_origin_name, "C smoke") != 0 ||
      strcmp(revision_origin_version, "1.0") != 0 ||
      revision_origin_uri != NULL ||
      strcmp(revision_message, "Import fixture") != 0) {
    pp_revision_set_release(revisions);
    pp_production_release(production);
    pp_error_release(error);
    return 36;
  }
  pp_revision_set_release(revisions);
  revisions = NULL;
  status = pp_production_changes_since(production, 0, 1, &revisions, &error);
  if (status != PP_OK || revisions == NULL ||
      pp_revision_set_count(revisions) != UINT64_C(1)) {
    pp_revision_set_release(revisions);
    pp_production_release(production);
    pp_error_release(error);
    return 37;
  }
  pp_revision_set_release(revisions);
  pp_revision_event_set_t *revision_events = NULL;
  status = pp_production_revision_events(production, &revision_id, &revision_events,
                                      &error);
  if (status != PP_OK || revision_events == NULL ||
      pp_revision_event_set_count(revision_events) != UINT64_C(8) ||
      pp_revision_event_set_get(revision_events, 0, &revision_event, &error) !=
          PP_OK ||
      revision_event.kind != PP_REVISION_ASSET_IMPORTED ||
      revision_event.position != UINT32_C(0) ||
      memcmp(revision_event.asset_id.bytes, asset_id.bytes,
             sizeof(asset_id.bytes)) != 0) {
    pp_revision_event_set_release(revision_events);
    pp_production_release(production);
    pp_error_release(error);
    return 38;
  }
  status = pp_revision_event_set_get(revision_events, 5, &revision_event,
                                     &error);
  if (status != PP_OK ||
      revision_event.kind != PP_REVISION_EXTERNAL_IDENTIFIER_ADDED ||
      revision_event.target.kind != PP_OBJECT_ASSET ||
      memcmp(revision_event.target.id.bytes, asset_id.bytes,
             sizeof(asset_id.bytes)) != 0 ||
      strcmp(revision_event.identifier_scheme, "com.example.asset") != 0 ||
      strcmp(revision_event.identifier_value, "asset-42") != 0 ||
      strcmp(revision_event.identifier_qualifier, "primary") != 0) {
    pp_revision_event_set_release(revision_events);
    pp_production_release(production);
    pp_error_release(error);
    return 39;
  }
  status = pp_revision_event_set_get(revision_events, 6, &revision_event,
                                     &error);
  if (status != PP_OK ||
      revision_event.kind != PP_REVISION_METADATA_ADDED_OR_REPLACED ||
      revision_event.target.kind != PP_OBJECT_ASSET ||
      strcmp(revision_event.vocabulary, "com.example.metadata") != 0 ||
      strcmp(revision_event.property, "title") != 0) {
    pp_revision_event_set_release(revision_events);
    pp_production_release(production);
    pp_error_release(error);
    return 40;
  }
  status = pp_revision_event_set_get(revision_events, 7, &revision_event,
                                     &error);
  if (status != PP_OK ||
      revision_event.kind != PP_REVISION_MEDIA_ROOT_ADDED ||
      memcmp(revision_event.media_root_id.bytes, root_id.bytes,
             sizeof(root_id.bytes)) != 0) {
    pp_revision_event_set_release(revision_events);
    pp_production_release(production);
    pp_error_release(error);
    return 41;
  }
  pp_revision_event_set_release(revision_events);
  asset_exists = 0;
  status = pp_production_asset_exists(production, &asset_id, &asset_exists, &error);
  if (status != PP_OK || asset_exists != UINT8_C(1)) {
    pp_production_release(production);
    pp_error_release(error);
    return 15;
  }
  pp_representation_set_t *representations = NULL;
  pp_uuid_t representation_asset_id = {{0}};
  pp_representation_kind_t representation_kind = 0;
  pp_content_structure_kind_t structure_kind = 0;
  uint64_t member_count = 0;
  uint64_t representation_resource_count = 0;
  uint64_t representation_fingerprint_count = 0;
  status = pp_production_representations(production, &asset_id,
                                         &representations, &error);
  if (status != PP_OK || representations == NULL ||
      pp_representation_set_count(representations) != UINT64_C(1) ||
      pp_representation_set_get(
          representations, 0, &representation_id, &representation_asset_id,
          &representation_kind, &structure_kind, &member_count,
          &representation_resource_count, &representation_fingerprint_count,
          &error) != PP_OK ||
      memcmp(representation_asset_id.bytes, asset_id.bytes,
             sizeof(asset_id.bytes)) != 0 ||
      representation_kind != PP_REPRESENTATION_ORIGINAL ||
      structure_kind != PP_CONTENT_SINGLE_RESOURCE ||
      member_count != UINT64_C(1) ||
      representation_resource_count != UINT64_C(1) ||
      representation_fingerprint_count != UINT64_C(1)) {
    pp_representation_set_release(representations);
    pp_production_release(production);
    pp_error_release(error);
    return 43;
  }
  const char *representation_fingerprint_algorithm = NULL;
  uint16_t representation_fingerprint_version = 0;
  const uint8_t *representation_fingerprint_value = NULL;
  uint64_t representation_fingerprint_value_length = 0;
  status = pp_representation_set_get_fingerprint(
      representations, 0, 0, &representation_fingerprint_algorithm,
      &representation_fingerprint_version, &representation_fingerprint_value,
      &representation_fingerprint_value_length, &error);
  if (status != PP_OK || representation_fingerprint_algorithm == NULL ||
      strcmp(representation_fingerprint_algorithm,
             "pp-blake3-representation") != 0 ||
      representation_fingerprint_version != UINT16_C(1) ||
      representation_fingerprint_value == NULL ||
      representation_fingerprint_value_length != UINT64_C(32)) {
    pp_representation_set_release(representations);
    pp_production_release(production);
    pp_error_release(error);
    return 48;
  }
  const char *member_role = NULL;
  uint8_t member_required = 0;
  status = pp_representation_set_get_member(
      representations, 0, 0, &resource_id, &member_role, &member_required,
      &error);
  if (status != PP_OK || uuid_is_zero(&resource_id) || member_role != NULL ||
      member_required != UINT8_C(1)) {
    pp_representation_set_release(representations);
    pp_production_release(production);
    pp_error_release(error);
    return 44;
  }
  pp_uuid_t inspected_resource_id = {{0}};
  uint8_t has_file_facts = 0;
  uint64_t file_size = 0;
  uint8_t has_modified_at = 0;
  int64_t modified_at = 0;
  uint64_t locator_count = 0;
  uint64_t resource_fingerprint_count = 0;
  status = pp_representation_set_get_resource(
      representations, 0, 0, &inspected_resource_id, &has_file_facts,
      &file_size, &has_modified_at, &modified_at, &locator_count,
      &resource_fingerprint_count, &error);
  if (status != PP_OK ||
      memcmp(inspected_resource_id.bytes, resource_id.bytes,
             sizeof(resource_id.bytes)) != 0 ||
      has_file_facts != UINT8_C(1) || file_size != UINT64_C(11) ||
      has_modified_at != UINT8_C(1) || modified_at == 0 ||
      locator_count != UINT64_C(1) ||
      resource_fingerprint_count != UINT64_C(1)) {
    pp_representation_set_release(representations);
    pp_production_release(production);
    pp_error_release(error);
    return 45;
  }
  const char *fingerprint_algorithm = NULL;
  uint16_t fingerprint_version = 0;
  const uint8_t *fingerprint_value = NULL;
  uint64_t fingerprint_value_length = 0;
  status = pp_representation_set_get_resource_fingerprint(
      representations, 0, 0, 0, &fingerprint_algorithm, &fingerprint_version,
      &fingerprint_value, &fingerprint_value_length, &error);
  if (status != PP_OK || fingerprint_algorithm == NULL ||
      fingerprint_version != UINT16_C(1) || fingerprint_value == NULL ||
      fingerprint_value_length == UINT64_C(0)) {
    pp_representation_set_release(representations);
    pp_production_release(production);
    pp_error_release(error);
    return 47;
  }
  pp_uuid_t locator_id = {{0}};
  const char *locator_uri = NULL;
  pp_locator_availability_t locator_availability = 0;
  uint8_t has_last_seen = 0;
  int64_t last_seen = 0;
  status = pp_representation_set_get_locator(
      representations, 0, 0, 0, &locator_id, &locator_uri,
      &locator_availability, &has_last_seen, &last_seen, &error);
  if (status != PP_OK || uuid_is_zero(&locator_id) || locator_uri == NULL ||
      locator_availability != PP_LOCATOR_ONLINE ||
      has_last_seen != UINT8_C(1) || last_seen == 0) {
    pp_representation_set_release(representations);
    pp_production_release(production);
    pp_error_release(error);
    return 46;
  }
  pp_representation_set_release(representations);
  pp_external_identifier_set_t *identifiers = NULL;
  status = pp_production_external_identifiers(production, &asset_ref, &identifiers,
                                           &error);
  const char *scheme = NULL;
  const char *external_value = NULL;
  const char *qualifier = NULL;
  if (status != PP_OK || identifiers == NULL ||
      pp_external_identifier_set_count(identifiers) != UINT64_C(1) ||
      pp_external_identifier_set_get(identifiers, 0, &scheme, &external_value,
                                     &qualifier, &error) != PP_OK ||
      strcmp(scheme, "com.example.asset") != 0 ||
      strcmp(external_value, "asset-42") != 0 ||
      strcmp(qualifier, "primary") != 0) {
    pp_external_identifier_set_release(identifiers);
    pp_production_release(production);
    pp_error_release(error);
    return 24;
  }
  pp_external_identifier_set_release(identifiers);
  pp_object_ref_set_t *objects = NULL;
  pp_object_ref_t found_object = {0, {{0}}};
  status = pp_production_find_by_external_identifier(
      production, "com.example.asset", "asset-42", &objects, &error);
  if (status != PP_OK || objects == NULL ||
      pp_object_ref_set_count(objects) != UINT64_C(1) ||
      pp_object_ref_set_get(objects, 0, &found_object, &error) != PP_OK ||
      found_object.kind != PP_OBJECT_ASSET ||
      memcmp(found_object.id.bytes, asset_id.bytes, sizeof(asset_id.bytes)) !=
          0) {
    pp_object_ref_set_release(objects);
    pp_production_release(production);
    pp_error_release(error);
    return 25;
  }
  pp_object_ref_set_release(objects);

  pp_metadata_set_t *metadata = NULL;
  status = pp_production_metadata(production, &asset_ref, &metadata, &error);
  pp_object_ref_t metadata_target = {0, {{0}}};
  const char *vocabulary = NULL;
  const char *property = NULL;
  const pp_metadata_value_t *metadata_value = NULL;
  const char *metadata_text = NULL;
  const char *metadata_language = NULL;
  if (status != PP_OK || metadata == NULL ||
      pp_metadata_set_count(metadata) != UINT64_C(1) ||
      pp_metadata_set_get(metadata, 0, &metadata_target, &vocabulary, &property,
                          &metadata_value, &error) != PP_OK ||
      metadata_target.kind != PP_OBJECT_ASSET ||
      strcmp(vocabulary, "com.example.metadata") != 0 ||
      strcmp(property, "title") != 0 ||
      pp_metadata_value_kind(metadata_value) != PP_METADATA_LANG_STRING ||
      pp_metadata_value_get_string(metadata_value, &metadata_text,
                                   &metadata_language, &error) != PP_OK ||
      strcmp(metadata_text, "C title") != 0 ||
      strcmp(metadata_language, "en-US") != 0) {
    pp_metadata_set_release(metadata);
    pp_production_release(production);
    pp_error_release(error);
    return 27;
  }
  pp_metadata_set_release(metadata);
  metadata = NULL;
  status = pp_production_find_metadata(production, "com.example.metadata", "title",
                                    &metadata, &error);
  if (status != PP_OK || metadata == NULL ||
      pp_metadata_set_count(metadata) != UINT64_C(1)) {
    pp_metadata_set_release(metadata);
    pp_production_release(production);
    pp_error_release(error);
    return 28;
  }
  pp_metadata_set_release(metadata);

  int moved_path_length =
      snprintf(moved_media_path, sizeof(moved_media_path), "%s.moved", argv[1]);
  if (moved_path_length < 0 ||
      (size_t)moved_path_length >= sizeof(moved_media_path) ||
      rename(media_path, moved_media_path) != 0) {
    pp_production_release(production);
    return 16;
  }

  pp_resolution_set_t *resolutions = NULL;
  status = pp_production_resolve_asset(production, &asset_id, &resolutions, &error);
  if (status != PP_OK || resolutions == NULL ||
      pp_resolution_set_representation_count(resolutions) != UINT64_C(1)) {
    pp_resolution_set_release(resolutions);
    pp_production_release(production);
    pp_error_release(error);
    return 17;
  }
  pp_representation_availability_t availability = 0;
  uint64_t resource_count = 0;
  uint64_t issue_count = 0;
  status = pp_resolution_set_get_representation(
      resolutions, 0, &representation_id, &availability, &resource_count,
      &issue_count, &error);
  if (status != PP_OK || availability != PP_AVAILABILITY_ONLINE ||
      resource_count != UINT64_C(1) || issue_count != 0 ||
      uuid_is_zero(&representation_id)) {
    pp_resolution_set_release(resolutions);
    pp_production_release(production);
    pp_error_release(error);
    return 18;
  }
  pp_resource_resolution_state_t state = 0;
  uint64_t candidate_count = 0;
  uint64_t result_evidence_count = 0;
  status = pp_resolution_set_get_resource(
      resolutions, 0, 0, &resource_id, &state, &candidate_count,
      &result_evidence_count, &error);
  if (status != PP_OK || state != PP_RESOURCE_RESOLVED_EXACT ||
      candidate_count != UINT64_C(1) || uuid_is_zero(&resource_id)) {
    pp_resolution_set_release(resolutions);
    pp_production_release(production);
    pp_error_release(error);
    return 18;
  }
  const char *candidate_uri = NULL;
  uint16_t confidence = 0;
  uint64_t candidate_evidence_count = 0;
  status = pp_resolution_set_get_candidate(
      resolutions, 0, 0, 0, &candidate_uri, &confidence,
      &candidate_evidence_count, &error);
  if (status != PP_OK || candidate_uri == NULL ||
      confidence != UINT16_C(10000) || candidate_evidence_count == 0) {
    pp_resolution_set_release(resolutions);
    pp_production_release(production);
    pp_error_release(error);
    return 19;
  }
  pp_evidence_kind_t evidence_kind = 0;
  const char *evidence_detail = NULL;
  status = pp_resolution_set_get_candidate_evidence(
      resolutions, 0, 0, 0, 0, &evidence_kind, &evidence_detail, &error);
  if (status != PP_OK || evidence_kind == 0) {
    pp_resolution_set_release(resolutions);
    pp_production_release(production);
    pp_error_release(error);
    return 20;
  }

  status = pp_production_begin_transaction(production, &transaction, &error);
  if (status != PP_OK ||
      pp_transaction_confirm_locator(transaction, &resource_id, candidate_uri,
                                     &error) != PP_OK ||
      pp_transaction_commit(transaction, &error) != PP_OK) {
    pp_transaction_release(transaction);
    pp_resolution_set_release(resolutions);
    pp_production_release(production);
    pp_error_release(error);
    return 21;
  }
  pp_transaction_release(transaction);
  transaction = NULL;

  status = pp_production_begin_transaction(production, &transaction, &error);
  if (status != PP_OK || transaction == NULL) {
    pp_resolution_set_release(resolutions);
    pp_production_release(production);
    pp_error_release(error);
    return 29;
  }
  const pp_activity_edge_t activity_output = {
      representation_id, "org.postproject:output.master"};
  const int64_t started_at = INT64_C(100);
  const int64_t finished_at = INT64_C(200);
  pp_uuid_t activity_id = {{0}};
  status = pp_transaction_create_activity(
      transaction, "org.postproject:ingest", NULL, 0, &activity_output, 1,
      &started_at, &finished_at, "C ingest", "1.0",
      "https://example.com/tools/ingest", "C operator", "com.example.agent",
      "operator-1", "primary", &activity_id, &error);
  pp_object_ref_t activity_ref = {PP_OBJECT_ACTIVITY, activity_id};
  if (status != PP_OK || uuid_is_zero(&activity_id)) {
    pp_transaction_release(transaction);
    pp_resolution_set_release(resolutions);
    pp_production_release(production);
    pp_error_release(error);
    return 30;
  }
  pp_metadata_input_t *rate_input = NULL;
  status = pp_metadata_input_create_rational(INT64_C(24000), UINT64_C(1001),
                                             &rate_input, &error);
  if (status == PP_OK) {
    status = pp_transaction_add_metadata_value(
        transaction, &activity_ref, "com.example.ingest", "rate", rate_input,
        &error);
  }
  pp_metadata_input_release(rate_input);
  if (status != PP_OK || pp_transaction_commit(transaction, &error) != PP_OK) {
    pp_transaction_release(transaction);
    pp_resolution_set_release(resolutions);
    pp_production_release(production);
    pp_error_release(error);
    return 30;
  }
  pp_transaction_release(transaction);
  transaction = NULL;

  metadata = NULL;
  status = pp_production_metadata(production, &activity_ref, &metadata, &error);
  int64_t rate_numerator = 0;
  uint64_t rate_denominator = 0;
  if (status != PP_OK || metadata == NULL ||
      pp_metadata_set_count(metadata) != UINT64_C(1) ||
      pp_metadata_set_get(metadata, 0, &metadata_target, &vocabulary, &property,
                          &metadata_value, &error) != PP_OK ||
      strcmp(vocabulary, "com.example.ingest") != 0 ||
      strcmp(property, "rate") != 0 ||
      pp_metadata_value_get_rational(metadata_value, &rate_numerator,
                                     &rate_denominator, &error) != PP_OK ||
      rate_numerator != INT64_C(24000) || rate_denominator != UINT64_C(1001)) {
    pp_metadata_set_release(metadata);
    pp_resolution_set_release(resolutions);
    pp_production_release(production);
    pp_error_release(error);
    return 30;
  }
  pp_metadata_set_release(metadata);

  pp_activity_set_t *activities = NULL;
  status = pp_production_activities(production, &activities, &error);
  pp_uuid_t read_activity_id = {{0}};
  const char *activity_kind = NULL;
  uint8_t has_started_at = 0;
  int64_t read_started_at = 0;
  uint8_t has_finished_at = 0;
  int64_t read_finished_at = 0;
  uint64_t input_count = 0;
  uint64_t output_count = 0;
  if (status != PP_OK || activities == NULL ||
      pp_activity_set_count(activities) != UINT64_C(1) ||
      pp_activity_set_get(activities, 0, &read_activity_id, &activity_kind,
                          &has_started_at, &read_started_at, &has_finished_at,
                          &read_finished_at, &input_count, &output_count,
                          &error) != PP_OK ||
      memcmp(read_activity_id.bytes, activity_id.bytes,
             sizeof(activity_id.bytes)) != 0 ||
      activity_kind == NULL || strcmp(activity_kind, "org.postproject:ingest") != 0 ||
      has_started_at != UINT8_C(1) || read_started_at != started_at ||
      has_finished_at != UINT8_C(1) || read_finished_at != finished_at ||
      input_count != 0 || output_count != UINT64_C(1)) {
    pp_activity_set_release(activities);
    pp_resolution_set_release(resolutions);
    pp_production_release(production);
    pp_error_release(error);
    return 31;
  }
  const char *tool_name = NULL;
  const char *tool_version = NULL;
  const char *tool_uri = NULL;
  const char *agent_name = NULL;
  const char *agent_scheme = NULL;
  const char *agent_value = NULL;
  const char *agent_qualifier = NULL;
  pp_uuid_t output_representation_id = {{0}};
  const char *output_role = NULL;
  if (pp_activity_set_get_tool(activities, 0, &tool_name, &tool_version,
                               &tool_uri, &error) != PP_OK ||
      tool_name == NULL || strcmp(tool_name, "C ingest") != 0 ||
      tool_version == NULL || strcmp(tool_version, "1.0") != 0 ||
      tool_uri == NULL ||
      strcmp(tool_uri, "https://example.com/tools/ingest") != 0 ||
      pp_activity_set_get_agent(
          activities, 0, &agent_name, &agent_scheme, &agent_value,
          &agent_qualifier, &error) != PP_OK ||
      agent_name == NULL || strcmp(agent_name, "C operator") != 0 ||
      agent_scheme == NULL || strcmp(agent_scheme, "com.example.agent") != 0 ||
      agent_value == NULL || strcmp(agent_value, "operator-1") != 0 ||
      agent_qualifier == NULL || strcmp(agent_qualifier, "primary") != 0 ||
      pp_activity_set_get_output(activities, 0, 0,
                                 &output_representation_id, &output_role,
                                 &error) != PP_OK ||
      memcmp(output_representation_id.bytes, representation_id.bytes,
             sizeof(representation_id.bytes)) != 0 ||
      output_role == NULL || strcmp(output_role, "org.postproject:output.master") != 0) {
    pp_activity_set_release(activities);
    pp_resolution_set_release(resolutions);
    pp_production_release(production);
    pp_error_release(error);
    return 32;
  }
  pp_activity_set_release(activities);
  activities = NULL;
  status = pp_production_activities_producing(
      production, &representation_id, &activities, &error);
  if (status != PP_OK || activities == NULL ||
      pp_activity_set_count(activities) != UINT64_C(1)) {
    pp_activity_set_release(activities);
    pp_resolution_set_release(resolutions);
    pp_production_release(production);
    pp_error_release(error);
    return 33;
  }
  pp_activity_set_release(activities);
  pp_resolution_set_release(resolutions);
  pp_production_release(production);
  production = NULL;

  status = pp_production_open(argv[1], &production, &error);
  if (status != PP_OK ||
      pp_production_resolve_asset(production, &asset_id, &resolutions, &error) !=
          PP_OK ||
      pp_resolution_set_get_representation(
          resolutions, 0, &representation_id, &availability, &resource_count,
          &issue_count, &error) != PP_OK ||
      pp_resolution_set_get_resource(
          resolutions, 0, 0, &resource_id, &state, &candidate_count,
          &result_evidence_count, &error) != PP_OK ||
      availability != PP_AVAILABILITY_ONLINE ||
      state != PP_RESOURCE_ONLINE_AT_KNOWN_LOCATOR) {
    pp_resolution_set_release(resolutions);
    pp_production_release(production);
    pp_error_release(error);
    return 22;
  }
  pp_resolution_set_release(resolutions);
  pp_production_release(production);
  production = NULL;

  status = pp_production_open(NULL, &production, &error);
  if (status != PP_ERROR_INVALID_ARGUMENT || error == NULL ||
      pp_error_code(error) != status || pp_error_message(error) == NULL) {
    pp_error_release(error);
    return 5;
  }
  pp_error_release(error);
  remove(moved_media_path);
  pp_production_release(NULL);
  pp_transaction_release(NULL);
  return 0;
}
