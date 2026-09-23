/* Runs every C listing included in the PostProject integrator guides.
 *
 * Each "[name]" ... "[/name]" region is included verbatim by the documentation
 * build, so keep regions self-contained and readable. Usage:
 *   postproject-c-guides WORK_DIRECTORY
 * The work directory is prepared by prepare-workdir.cmake.
 */
#include <postproject/postproject.h>

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* [create-production] */
static pp_error_code_t create_production(const char *path, const char *media,
                                         pp_production_t **out_production,
                                         pp_uuid_t *out_asset_id,
                                         pp_error_t **error) {
  pp_production_t *production = NULL;
  pp_transaction_t *transaction = NULL;
  pp_representation_set_t *representations = NULL;

  pp_error_code_t status =
      pp_production_create(path, "Documentary", &production, error);
  if (status == PP_OK) {
    status = pp_production_begin_transaction(production, &transaction, error);
  }
  if (status == PP_OK) {
    status = pp_transaction_set_revision_context(
        transaction, "com.example.editor", "0.4.0", NULL,
        "Import camera original", error);
  }
  if (status == PP_OK) {
    status = pp_transaction_import_media(transaction, media, "Camera A",
                                         out_asset_id, error);
  }
  if (status == PP_OK) {
    status = pp_transaction_commit(transaction, error);
  }
  if (status == PP_OK) {
    status = pp_production_representations(production, out_asset_id,
                                           &representations, error);
  }
  if (status == PP_OK) {
    printf("representations: %llu\n",
           (unsigned long long)pp_representation_set_count(representations));
    *out_production = production;
    production = NULL;
  }

  pp_representation_set_release(representations);
  pp_transaction_release(transaction);
  pp_production_release(production);
  return status;
}
/* [/create-production] */

/* [external-identifiers] */
static pp_error_code_t tag_camera_serial(pp_production_t *production,
                                         const pp_uuid_t *asset_id,
                                         pp_error_t **error) {
  const pp_object_ref_t target = {PP_OBJECT_ASSET, *asset_id};
  pp_transaction_t *transaction = NULL;
  pp_external_identifier_set_t *attached = NULL;
  pp_object_ref_set_t *matches = NULL;

  pp_error_code_t status =
      pp_production_begin_transaction(production, &transaction, error);
  if (status == PP_OK) {
    status = pp_transaction_add_external_identifier(
        transaction, &target, "com.example.camera.serial", "A-0007", NULL,
        error);
  }
  if (status == PP_OK) {
    status = pp_transaction_commit(transaction, error);
  }
  if (status == PP_OK) {
    status = pp_production_external_identifiers(production, &target,
                                                &attached, error);
  }
  if (status == PP_OK) {
    status = pp_production_find_by_external_identifier(
        production, "com.example.camera.serial", "A-0007", &matches, error);
  }
  if (status == PP_OK) {
    printf("identifiers: %llu, matching objects: %llu\n",
           (unsigned long long)pp_external_identifier_set_count(attached),
           (unsigned long long)pp_object_ref_set_count(matches));
  }

  pp_object_ref_set_release(matches);
  pp_external_identifier_set_release(attached);
  pp_transaction_release(transaction);
  return status;
}
/* [/external-identifiers] */

/* [metadata] */
static const char *const IPTC_VIDEO_METADATA_HUB =
    "https://iptc.org/std/videometadatahub/recommendation/"
    "iptc-vmhub-1.7-schema.json";

static pp_error_code_t add_title(pp_production_t *production,
                                 const pp_uuid_t *asset_id,
                                 pp_error_t **error) {
  const pp_object_ref_t target = {PP_OBJECT_ASSET, *asset_id};
  pp_metadata_input_t *title = NULL;
  pp_transaction_t *transaction = NULL;
  pp_metadata_set_t *on_asset = NULL;
  pp_metadata_set_t *everywhere = NULL;

  pp_error_code_t status =
      pp_metadata_input_create_string("Interview", "en-US", &title, error);
  if (status == PP_OK) {
    status = pp_production_begin_transaction(production, &transaction, error);
  }
  if (status == PP_OK) {
    status = pp_transaction_add_metadata_value(
        transaction, &target, IPTC_VIDEO_METADATA_HUB, "title", title, error);
  }
  if (status == PP_OK) {
    status = pp_transaction_commit(transaction, error);
  }
  if (status == PP_OK) {
    status = pp_production_metadata(production, &target, &on_asset, error);
  }
  if (status == PP_OK) {
    status = pp_production_find_metadata(production, IPTC_VIDEO_METADATA_HUB,
                                         "title", &everywhere, error);
  }
  for (uint64_t index = 0;
       status == PP_OK && index < pp_metadata_set_count(on_asset); ++index) {
    pp_object_ref_t owner;
    const char *vocabulary = NULL;
    const char *property = NULL;
    const pp_metadata_value_t *value = NULL;
    const char *text = NULL;
    const char *language = NULL;
    status = pp_metadata_set_get(on_asset, index, &owner, &vocabulary,
                                 &property, &value, error);
    if (status == PP_OK &&
        pp_metadata_value_kind(value) == PP_METADATA_LANG_STRING) {
      status = pp_metadata_value_get_string(value, &text, &language, error);
      if (status == PP_OK) {
        printf("%s: %s [%s]\n", property, text, language);
      }
    }
  }

  pp_metadata_set_release(everywhere);
  pp_metadata_set_release(on_asset);
  pp_transaction_release(transaction);
  pp_metadata_input_release(title);
  return status;
}
/* [/metadata] */

/* [media-root] */
static pp_error_code_t add_rushes_root(pp_production_t *production,
                                       pp_error_t **error) {
  pp_transaction_t *transaction = NULL;
  pp_uuid_t root_id;

  pp_error_code_t status =
      pp_production_begin_transaction(production, &transaction, error);
  if (status == PP_OK) {
    status = pp_transaction_add_media_root(transaction, "rushes",
                                           "Camera originals", 0, &root_id,
                                           error);
  }
  if (status == PP_OK) {
    status = pp_transaction_commit(transaction, error);
  }

  pp_transaction_release(transaction);
  return status;
}
/* [/media-root] */

/* [resolve-asset] */
static pp_error_code_t resolve_asset(const pp_production_t *production,
                                     const pp_uuid_t *asset_id,
                                     const char *rushes_directory,
                                     pp_resolution_set_t **out_resolutions,
                                     pp_error_t **error) {
  const pp_media_root_mapping_t mappings[] = {{"rushes", rushes_directory}};
  pp_resolution_set_t *resolutions = NULL;

  pp_error_code_t status = pp_production_resolve_asset(
      production, asset_id, mappings, 1, &resolutions, error);
  const uint64_t count =
      status == PP_OK ? pp_resolution_set_representation_count(resolutions) : 0;
  for (uint64_t r = 0; status == PP_OK && r < count; ++r) {
    pp_uuid_t representation_id;
    pp_representation_availability_t availability;
    uint64_t resource_count = 0;
    uint64_t issue_count = 0;
    status = pp_resolution_set_get_representation(
        resolutions, r, &representation_id, &availability, &resource_count,
        &issue_count, error);
    if (status == PP_OK) {
      printf("availability: %u\n", availability);
    }
    for (uint64_t s = 0; status == PP_OK && s < resource_count; ++s) {
      pp_uuid_t resource_id;
      pp_resource_resolution_state_t state;
      uint64_t candidate_count = 0;
      uint64_t evidence_count = 0;
      status = pp_resolution_set_get_resource(resolutions, r, s, &resource_id,
                                              &state, &candidate_count,
                                              &evidence_count, error);
      for (uint64_t c = 0; status == PP_OK && c < candidate_count; ++c) {
        const char *uri = NULL;
        uint16_t confidence = 0;
        status = pp_resolution_set_get_candidate(resolutions, r, s, c, &uri,
                                                 &confidence, &evidence_count,
                                                 error);
        if (status == PP_OK) {
          printf("candidate: %s (%u/10000)\n", uri, confidence);
        }
      }
    }
  }

  if (status == PP_OK) {
    *out_resolutions = resolutions;
  } else {
    pp_resolution_set_release(resolutions);
  }
  return status;
}
/* [/resolve-asset] */

/* [confirm-locator] */
static pp_error_code_t
confirm_unique_candidates(pp_production_t *production,
                          const pp_resolution_set_t *resolutions,
                          pp_error_t **error) {
  pp_transaction_t *transaction = NULL;
  pp_error_code_t status =
      pp_production_begin_transaction(production, &transaction, error);

  const uint64_t count = pp_resolution_set_representation_count(resolutions);
  for (uint64_t r = 0; status == PP_OK && r < count; ++r) {
    pp_uuid_t representation_id;
    pp_representation_availability_t availability;
    uint64_t resource_count = 0;
    uint64_t issue_count = 0;
    status = pp_resolution_set_get_representation(
        resolutions, r, &representation_id, &availability, &resource_count,
        &issue_count, error);
    for (uint64_t s = 0; status == PP_OK && s < resource_count; ++s) {
      pp_uuid_t resource_id;
      pp_resource_resolution_state_t state;
      uint64_t candidate_count = 0;
      uint64_t evidence_count = 0;
      const char *uri = NULL;
      uint16_t confidence = 0;
      status = pp_resolution_set_get_resource(resolutions, r, s, &resource_id,
                                              &state, &candidate_count,
                                              &evidence_count, error);
      /* Several candidates need a person to choose; never pick one here. */
      if (status == PP_OK && candidate_count == 1) {
        status = pp_resolution_set_get_candidate(resolutions, r, s, 0, &uri,
                                                 &confidence, &evidence_count,
                                                 error);
      }
      if (status == PP_OK && uri != NULL) {
        status = pp_transaction_confirm_locator(transaction, &resource_id, uri,
                                                error);
      }
    }
  }
  if (status == PP_OK) {
    status = pp_transaction_commit(transaction, error);
  }

  pp_transaction_release(transaction);
  return status;
}
/* [/confirm-locator] */

/* [image-sequence] */
static pp_error_code_t add_render_sequence(pp_production_t *production,
                                           const pp_uuid_t *asset_id,
                                           const char *directory,
                                           pp_uuid_t *out_sequence_id,
                                           pp_error_t **error) {
  const int64_t missing_frames[] = {1003};
  pp_transaction_t *transaction = NULL;
  pp_representation_set_t *representations = NULL;

  pp_error_code_t status =
      pp_production_begin_transaction(production, &transaction, error);
  if (status == PP_OK) {
    status = pp_transaction_add_image_sequence_representation(
        transaction, asset_id, PP_REPRESENTATION_DERIVED, directory,
        "shot010.", ".exr", 4, 1001, 1004, 1, 24000, 1001, missing_frames, 1,
        out_sequence_id, error);
  }
  if (status == PP_OK) {
    status = pp_transaction_commit(transaction, error);
  }
  if (status == PP_OK) {
    status = pp_production_representations(production, asset_id,
                                           &representations, error);
  }
  for (uint64_t index = 0;
       status == PP_OK && index < pp_representation_set_count(representations);
       ++index) {
    pp_uuid_t id;
    pp_uuid_t owner;
    pp_representation_kind_t kind;
    pp_content_structure_kind_t structure;
    uint64_t members = 0;
    uint64_t resources = 0;
    uint64_t fingerprints = 0;
    status = pp_representation_set_get(representations, index, &id, &owner,
                                       &kind, &structure, &members, &resources,
                                       &fingerprints, error);
    if (status == PP_OK && structure == PP_CONTENT_IMAGE_SEQUENCE) {
      const char *prefix = NULL;
      const char *suffix = NULL;
      uint8_t padding = 0;
      int64_t start = 0;
      int64_t end = 0;
      uint32_t step = 0;
      uint32_t rate_numerator = 0;
      uint32_t rate_denominator = 0;
      uint64_t missing_count = 0;
      status = pp_representation_set_get_sequence(
          representations, index, &prefix, &suffix, &padding, &start, &end,
          &step, &rate_numerator, &rate_denominator, &missing_count, error);
      if (status == PP_OK) {
        printf("%s#%s frames %lld-%lld, %llu known missing\n", prefix, suffix,
               (long long)start, (long long)end,
               (unsigned long long)missing_count);
      }
    }
  }

  pp_representation_set_release(representations);
  pp_transaction_release(transaction);
  return status;
}
/* [/image-sequence] */

/* [provenance] */
static pp_error_code_t record_render(pp_production_t *production,
                                     const pp_uuid_t *source_id,
                                     const pp_uuid_t *render_id,
                                     pp_error_t **error) {
  const pp_activity_edge_t inputs[] = {{*source_id, "org.postproject:primary"}};
  const pp_activity_edge_t outputs[] = {{*render_id, NULL}};
  pp_transaction_t *transaction = NULL;
  pp_activity_set_t *producers = NULL;
  pp_object_ref_set_t *ancestors = NULL;
  pp_uuid_t activity_id;

  pp_error_code_t status =
      pp_production_begin_transaction(production, &transaction, error);
  if (status == PP_OK) {
    status = pp_transaction_create_activity(
        transaction, "org.postproject:render", inputs, 1, outputs, 1, NULL,
        NULL, "Example Renderer", "2.1", "https://example.com/renderer", NULL,
        NULL, NULL, NULL, &activity_id, error);
  }
  if (status == PP_OK) {
    status = pp_transaction_commit(transaction, error);
  }
  if (status == PP_OK) {
    status = pp_production_activities_producing(production, render_id,
                                                &producers, error);
  }
  if (status == PP_OK) {
    status = pp_production_provenance_ancestors(production, render_id,
                                                &ancestors, error);
  }
  if (status == PP_OK) {
    printf("producers: %llu, ancestors: %llu\n",
           (unsigned long long)pp_activity_set_count(producers),
           (unsigned long long)pp_object_ref_set_count(ancestors));
  }

  pp_object_ref_set_release(ancestors);
  pp_activity_set_release(producers);
  pp_transaction_release(transaction);
  return status;
}
/* [/provenance] */

static void handle_event(const pp_revision_event_t *event) {
  printf("event %u: kind %u\n", event->position, event->kind);
}

/* [revision-feed] */
static pp_error_code_t process_changes(const pp_production_t *production,
                                       uint64_t *cursor, pp_error_t **error) {
  const uint32_t limit = 100;
  pp_error_code_t status = PP_OK;
  uint64_t page_size = limit;

  while (status == PP_OK && page_size == limit) {
    pp_revision_set_t *page = NULL;
    status = pp_production_changes_since(production, *cursor, limit, &page,
                                         error);
    page_size = status == PP_OK ? pp_revision_set_count(page) : 0;
    for (uint64_t i = 0; status == PP_OK && i < page_size; ++i) {
      pp_uuid_t revision_id;
      pp_uuid_t transaction_id;
      uint64_t sequence = 0;
      int64_t committed_at = 0;
      const char *origin_name, *origin_version, *origin_uri, *message;
      pp_revision_event_set_t *events = NULL;
      status = pp_revision_set_get(page, i, &revision_id, &sequence,
                                   &transaction_id, &committed_at,
                                   &origin_name, &origin_version, &origin_uri,
                                   &message, error);
      if (status == PP_OK) {
        status = pp_production_revision_events(production, &revision_id,
                                               &events, error);
      }
      for (uint64_t j = 0;
           status == PP_OK && j < pp_revision_event_set_count(events); ++j) {
        pp_revision_event_t event;
        status = pp_revision_event_set_get(events, j, &event, error);
        if (status == PP_OK) {
          /* Switch on event.kind; unused fields are zero or NULL. */
          handle_event(&event);
        }
      }
      pp_revision_event_set_release(events);
      if (status == PP_OK) {
        /* Persist the cursor only after the whole revision is processed. */
        *cursor = sequence;
      }
    }
    pp_revision_set_release(page);
  }
  return status;
}
/* [/revision-feed] */

/* [host-binding] */
static pp_error_code_t bind_representation(const pp_uuid_t *production_id,
                                           const pp_uuid_t *representation_id,
                                           pp_error_t **error) {
  const pp_object_ref_t object = {PP_OBJECT_REPRESENTATION,
                                  *representation_id};
  char *stored = NULL;
  pp_uuid_t parsed_production = {{0}};
  pp_object_ref_t parsed_object = {0, {{0}}};

  pp_error_code_t status =
      pp_host_binding_format(production_id, &object, &stored, error);
  if (status == PP_OK) {
    printf("binding: %s\n", stored);
    status = pp_host_binding_parse(stored, &parsed_production, &parsed_object,
                                   error);
  }
  if (status == PP_OK &&
      (memcmp(&parsed_production, production_id, sizeof *production_id) != 0 ||
       parsed_object.kind != object.kind ||
       memcmp(&parsed_object.id, &object.id, sizeof object.id) != 0)) {
    status = PP_ERROR_INTERNAL;
  }

  pp_host_binding_release(stored);
  return status;
}
/* [/host-binding] */

static void join(char *buffer, size_t size, const char *directory,
                 const char *name) {
  snprintf(buffer, size, "%s/%s", directory, name);
}

static pp_error_code_t
original_representation(const pp_production_t *production,
                        const pp_uuid_t *asset_id, pp_uuid_t *out_id,
                        pp_error_t **error) {
  pp_representation_set_t *representations = NULL;
  pp_error_code_t status =
      pp_production_representations(production, asset_id, &representations,
                                    error);
  if (status == PP_OK) {
    pp_uuid_t owner;
    pp_representation_kind_t kind;
    pp_content_structure_kind_t structure;
    uint64_t members, resources, fingerprints;
    status = pp_representation_set_get(representations, 0, out_id, &owner,
                                       &kind, &structure, &members, &resources,
                                       &fingerprints, error);
  }
  pp_representation_set_release(representations);
  return status;
}

int main(int argc, char **argv) {
  if (argc != 2) {
    fprintf(stderr, "usage: postproject-c-guides WORK_DIRECTORY\n");
    return 2;
  }
  const char *work = argv[1];
  char production_path[4096];
  char media[4096];
  char moved_media[4096];
  char moved[4096];
  char renders[4096];
  join(production_path, sizeof production_path, work, "production.pproj");
  join(media, sizeof media, work, "rushes/A001.mov");
  join(moved_media, sizeof moved_media, work, "moved/A001.mov");
  join(moved, sizeof moved, work, "moved");
  join(renders, sizeof renders, work, "renders/shot010");

  pp_production_t *production = NULL;
  pp_resolution_set_t *resolutions = NULL;
  pp_error_t *error = NULL;
  pp_uuid_t asset_id;
  pp_uuid_t production_id;
  pp_uuid_t original_id;
  pp_uuid_t sequence_id;
  uint64_t cursor = 0;

  pp_error_code_t status = create_production(production_path, media,
                                             &production, &asset_id, &error);
  if (status == PP_OK) {
    status = original_representation(production, &asset_id, &original_id,
                                     &error);
  }
  if (status == PP_OK) {
    status = tag_camera_serial(production, &asset_id, &error);
  }
  if (status == PP_OK) {
    status = add_title(production, &asset_id, &error);
  }
  if (status == PP_OK) {
    status = add_rushes_root(production, &error);
  }
  if (status == PP_OK && rename(media, moved_media) != 0) {
    perror("move media");
    status = PP_ERROR_IO;
  }
  if (status == PP_OK) {
    status = resolve_asset(production, &asset_id, moved, &resolutions, &error);
  }
  if (status == PP_OK) {
    status = confirm_unique_candidates(production, resolutions, &error);
  }
  if (status == PP_OK) {
    status = add_render_sequence(production, &asset_id, renders, &sequence_id,
                                 &error);
  }
  if (status == PP_OK) {
    status = record_render(production, &original_id, &sequence_id, &error);
  }
  if (status == PP_OK) {
    status = process_changes(production, &cursor, &error);
  }
  if (status == PP_OK && cursor == 0) {
    status = PP_ERROR_INTERNAL;
  }
  if (status == PP_OK) {
    status = pp_production_id(production, &production_id, &error);
  }
  if (status == PP_OK) {
    status = bind_representation(&production_id, &sequence_id, &error);
  }

  if (status != PP_OK) {
    fprintf(stderr, "operation failed (%u): %s\n", status,
            error != NULL ? pp_error_message(error) : "no details");
  }
  pp_error_release(error);
  pp_resolution_set_release(resolutions);
  pp_production_release(production);
  return status == PP_OK ? EXIT_SUCCESS : EXIT_FAILURE;
}
