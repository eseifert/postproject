#include <postproject/postproject.h>

#include <stdio.h>

static void print_uuid(const pp_uuid_t *id) {
  for (size_t index = 0; index < sizeof(id->bytes); ++index) {
    printf("%02x", id->bytes[index]);
  }
  putchar('\n');
}

int main(int argc, char **argv) {
  if (argc != 3) {
    fprintf(stderr, "usage: postproject-c-example PROJECT MEDIA\n");
    return 2;
  }

  pp_project_t *project = NULL;
  pp_transaction_t *transaction = NULL;
  pp_error_t *error = NULL;
  pp_uuid_t asset_id = {{0}};
  pp_error_code_t status = pp_project_open(argv[1], &project, &error);
  if (status == PP_OK) {
    status = pp_project_begin_transaction(project, &transaction, &error);
  }
  if (status == PP_OK) {
    status = pp_transaction_import_media(transaction, argv[2], NULL, &asset_id,
                                         &error);
  }
  if (status == PP_OK) {
    status = pp_transaction_commit(transaction, &error);
  }

  if (status != PP_OK) {
    fprintf(stderr, "operation failed (%u): %s\n", status,
            error != NULL ? pp_error_message(error) : "no details");
  } else {
    print_uuid(&asset_id);
  }
  pp_error_release(error);
  pp_transaction_release(transaction);
  pp_project_release(project);
  return status == PP_OK ? 0 : 1;
}
