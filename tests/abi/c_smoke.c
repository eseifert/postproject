#include <postproject/postproject.h>

#include <stdint.h>
#include <stdio.h>
#include <string.h>

static int uuid_is_zero(const pp_uuid_t *id) {
  static const uint8_t zero[16] = {0};
  return memcmp(id->bytes, zero, sizeof(zero)) == 0;
}

int main(int argc, char **argv) {
  pp_project_t *project = NULL;
  pp_transaction_t *transaction = NULL;
  pp_error_t *error = NULL;
  pp_uuid_t id = {{0}};
  pp_uuid_t asset_id = {{0}};
  pp_uuid_t rolled_back_asset_id = {{0}};
  pp_uuid_t root_id = {{0}};
  pp_uuid_t representation_id = {{0}};
  char media_path[4096];
  char moved_media_path[4096];

  if (argc != 3) {
    return 64;
  }
  if (pp_abi_version() != UINT32_C(3)) {
    return 1;
  }
  pp_error_code_t status =
      pp_project_create(argv[1], "C smoke test", &project, &error);
  if (status != PP_OK) {
    fprintf(stderr, "create failed (%u): %s\n", status,
            error != NULL ? pp_error_message(error) : "no details");
    pp_error_release(error);
    return 2;
  }
  if (pp_project_id(project, &id, &error) != PP_OK || uuid_is_zero(&id)) {
    pp_project_release(project);
    pp_error_release(error);
    return 3;
  }
  int media_path_length =
      snprintf(media_path, sizeof(media_path), "%s.media", argv[1]);
  if (media_path_length < 0 ||
      (size_t)media_path_length >= sizeof(media_path)) {
    pp_project_release(project);
    return 6;
  }
  FILE *media = fopen(media_path, "wb");
  if (media == NULL) {
    pp_project_release(project);
    return 7;
  }
  size_t written = fwrite("C ABI media", 1, 11, media);
  int close_status = fclose(media);
  if (written != 11 || close_status != 0) {
    pp_project_release(project);
    return 7;
  }
  status = pp_project_begin_transaction(project, &transaction, &error);
  if (status != PP_OK || transaction == NULL) {
    pp_project_release(project);
    pp_error_release(error);
    return 8;
  }
  status = pp_transaction_import_media(transaction, media_path, NULL,
                                       &rolled_back_asset_id, &error);
  if (status != PP_OK || uuid_is_zero(&rolled_back_asset_id) ||
      pp_transaction_rollback(transaction, &error) != PP_OK) {
    pp_transaction_release(transaction);
    pp_project_release(project);
    pp_error_release(error);
    return 9;
  }
  pp_transaction_release(transaction);
  transaction = NULL;
  uint8_t asset_exists = 1;
  status = pp_project_asset_exists(project, &rolled_back_asset_id,
                                   &asset_exists, &error);
  if (status != PP_OK || asset_exists != UINT8_C(0)) {
    pp_project_release(project);
    pp_error_release(error);
    return 10;
  }

  status = pp_project_begin_transaction(project, &transaction, &error);
  if (status != PP_OK || transaction == NULL) {
    pp_project_release(project);
    pp_error_release(error);
    return 11;
  }
  status = pp_transaction_import_media(transaction, media_path, "C asset",
                                       &asset_id, &error);
  if (status != PP_OK || uuid_is_zero(&asset_id)) {
    pp_transaction_release(transaction);
    pp_project_release(project);
    pp_error_release(error);
    return 12;
  }
  pp_object_ref_t asset_ref = {PP_OBJECT_ASSET, asset_id};
  status = pp_transaction_add_external_identifier(
      transaction, &asset_ref, "com.example.asset", "asset-42", "primary",
      &error);
  if (status != PP_OK) {
    pp_transaction_release(transaction);
    pp_project_release(project);
    pp_error_release(error);
    return 23;
  }
  status = pp_transaction_add_media_root(transaction, argv[2], "fixture root",
                                         0, &root_id, &error);
  if (status != PP_OK || uuid_is_zero(&root_id)) {
    pp_transaction_release(transaction);
    pp_project_release(project);
    pp_error_release(error);
    return 13;
  }
  status = pp_transaction_commit(transaction, &error);
  if (status != PP_OK) {
    pp_transaction_release(transaction);
    pp_project_release(project);
    pp_error_release(error);
    return 14;
  }
  pp_transaction_release(transaction);
  pp_project_release(project);
  project = NULL;

  status = pp_project_open(argv[1], &project, &error);
  if (status != PP_OK) {
    fprintf(stderr, "open failed (%u): %s\n", status,
            error != NULL ? pp_error_message(error) : "no details");
    pp_error_release(error);
    return 4;
  }
  asset_exists = 0;
  status = pp_project_asset_exists(project, &asset_id, &asset_exists, &error);
  if (status != PP_OK || asset_exists != UINT8_C(1)) {
    pp_project_release(project);
    pp_error_release(error);
    return 15;
  }
  pp_external_identifier_set_t *identifiers = NULL;
  status = pp_project_external_identifiers(project, &asset_ref, &identifiers,
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
    pp_project_release(project);
    pp_error_release(error);
    return 24;
  }
  pp_external_identifier_set_release(identifiers);
  pp_object_ref_set_t *objects = NULL;
  pp_object_ref_t found_object = {0, {{0}}};
  status = pp_project_find_by_external_identifier(
      project, "com.example.asset", "asset-42", &objects, &error);
  if (status != PP_OK || objects == NULL ||
      pp_object_ref_set_count(objects) != UINT64_C(1) ||
      pp_object_ref_set_get(objects, 0, &found_object, &error) != PP_OK ||
      found_object.kind != PP_OBJECT_ASSET ||
      memcmp(found_object.id.bytes, asset_id.bytes, sizeof(asset_id.bytes)) !=
          0) {
    pp_object_ref_set_release(objects);
    pp_project_release(project);
    pp_error_release(error);
    return 25;
  }
  pp_object_ref_set_release(objects);

  int moved_path_length =
      snprintf(moved_media_path, sizeof(moved_media_path), "%s.moved", argv[1]);
  if (moved_path_length < 0 ||
      (size_t)moved_path_length >= sizeof(moved_media_path) ||
      rename(media_path, moved_media_path) != 0) {
    pp_project_release(project);
    return 16;
  }

  pp_resolution_set_t *resolutions = NULL;
  status = pp_project_resolve_asset(project, &asset_id, &resolutions, &error);
  if (status != PP_OK || resolutions == NULL ||
      pp_resolution_set_count(resolutions) != UINT64_C(1)) {
    pp_resolution_set_release(resolutions);
    pp_project_release(project);
    pp_error_release(error);
    return 17;
  }
  pp_resolution_state_t state = 0;
  uint64_t candidate_count = 0;
  uint64_t result_evidence_count = 0;
  status = pp_resolution_set_get(resolutions, 0, &representation_id, &state,
                                 &candidate_count, &result_evidence_count,
                                 &error);
  if (status != PP_OK || state != PP_RESOLUTION_RESOLVED_EXACT ||
      candidate_count != UINT64_C(1) || uuid_is_zero(&representation_id)) {
    pp_resolution_set_release(resolutions);
    pp_project_release(project);
    pp_error_release(error);
    return 18;
  }
  const char *candidate_uri = NULL;
  uint16_t confidence = 0;
  uint64_t candidate_evidence_count = 0;
  status = pp_resolution_candidate_get(
      resolutions, 0, 0, &candidate_uri, &confidence,
      &candidate_evidence_count, &error);
  if (status != PP_OK || candidate_uri == NULL ||
      confidence != UINT16_C(10000) || candidate_evidence_count == 0) {
    pp_resolution_set_release(resolutions);
    pp_project_release(project);
    pp_error_release(error);
    return 19;
  }
  pp_evidence_kind_t evidence_kind = 0;
  const char *evidence_detail = NULL;
  status = pp_resolution_candidate_evidence_get(
      resolutions, 0, 0, 0, &evidence_kind, &evidence_detail, &error);
  if (status != PP_OK || evidence_kind == 0) {
    pp_resolution_set_release(resolutions);
    pp_project_release(project);
    pp_error_release(error);
    return 20;
  }

  status = pp_project_begin_transaction(project, &transaction, &error);
  if (status != PP_OK ||
      pp_transaction_confirm_location(transaction, &representation_id,
                                      candidate_uri, &error) != PP_OK ||
      pp_transaction_commit(transaction, &error) != PP_OK) {
    pp_transaction_release(transaction);
    pp_resolution_set_release(resolutions);
    pp_project_release(project);
    pp_error_release(error);
    return 21;
  }
  pp_transaction_release(transaction);
  transaction = NULL;
  pp_resolution_set_release(resolutions);
  pp_project_release(project);
  project = NULL;

  status = pp_project_open(argv[1], &project, &error);
  if (status != PP_OK ||
      pp_project_resolve_asset(project, &asset_id, &resolutions, &error) !=
          PP_OK ||
      pp_resolution_set_get(resolutions, 0, &representation_id, &state,
                            &candidate_count, &result_evidence_count,
                            &error) != PP_OK ||
      state != PP_RESOLUTION_ONLINE_AT_KNOWN_LOCATION) {
    pp_resolution_set_release(resolutions);
    pp_project_release(project);
    pp_error_release(error);
    return 22;
  }
  pp_resolution_set_release(resolutions);
  pp_project_release(project);
  project = NULL;

  status = pp_project_open(NULL, &project, &error);
  if (status != PP_ERROR_INVALID_ARGUMENT || error == NULL ||
      pp_error_code(error) != status || pp_error_message(error) == NULL) {
    pp_error_release(error);
    return 5;
  }
  pp_error_release(error);
  remove(moved_media_path);
  pp_project_release(NULL);
  pp_transaction_release(NULL);
  return 0;
}
