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
    pp_error_t *error = NULL;
    pp_uuid_t id = {{0}};

    if (argc != 2) {
        return 64;
    }
    if (pp_abi_version() != UINT32_C(1)) {
        return 1;
    }
    pp_error_code_t status = pp_project_create(argv[1], "C smoke test", &project, &error);
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
    pp_project_release(project);
    project = NULL;

    status = pp_project_open(argv[1], &project, &error);
    if (status != PP_OK) {
        fprintf(stderr, "open failed (%u): %s\n", status,
                error != NULL ? pp_error_message(error) : "no details");
        pp_error_release(error);
        return 4;
    }
    pp_project_release(project);

    status = pp_project_open(NULL, &project, &error);
    if (status != PP_ERROR_INVALID_ARGUMENT || error == NULL ||
        pp_error_code(error) != status || pp_error_message(error) == NULL) {
        pp_error_release(error);
        return 5;
    }
    pp_error_release(error);
    pp_project_release(NULL);
    return 0;
}

