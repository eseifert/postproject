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
  char media_path[4096];

  if (argc != 2) {
    return 64;
  }
  if (pp_abi_version() != UINT32_C(1)) {
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
  status = pp_transaction_add_media_root(transaction, ".", "working directory",
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
  pp_project_release(project);

  status = pp_project_open(NULL, &project, &error);
  if (status != PP_ERROR_INVALID_ARGUMENT || error == NULL ||
      pp_error_code(error) != status || pp_error_message(error) == NULL) {
    pp_error_release(error);
    return 5;
  }
  pp_error_release(error);
  remove(media_path);
  pp_project_release(NULL);
  pp_transaction_release(NULL);
  return 0;
}
