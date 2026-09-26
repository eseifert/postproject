// Runs the C++ listings about requesting, claiming, and finishing jobs.
//
// Each "[name]" ... "[/name]" region is included verbatim by the documentation
// build, so keep regions self-contained and readable. Usage:
//   postproject-cpp-jobs WORK_DIRECTORY
// The work directory is prepared by prepare-workdir.cmake.
#include <postproject/postproject.hpp>

#include <cstdint>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <vector>

namespace {

void require(bool condition, const char *message) {
  if (!condition) {
    throw std::runtime_error(message);
  }
}

// Timestamps are supplied by the caller; these fixed values stand in for a
// clock reading in microseconds since the Unix epoch.
constexpr std::int64_t t0 = 1'700'000'000'000'000;
constexpr std::int64_t one_minute = 60'000'000;

const postproject::ToolIdentity encoder{"Example Encoder", "5.1", std::nullopt};
const postproject::AgentIdentity worker{
    "render-node-04", postproject::ExternalIdentifier{"com.example.farm.node",
                                                      "04", std::nullopt}};

postproject::Job load_job(const postproject::Production &production,
                          const postproject::Uuid &job_id) {
  std::optional<std::string> cursor;
  do {
    const auto page = production.jobs(100, cursor);
    for (const auto &job : page.items) {
      if (job.id == job_id) {
        return job;
      }
    }
    cursor = page.next_cursor;
  } while (cursor.has_value());
  throw std::runtime_error("job not found");
}

// [request-job]
constexpr const char *transcode = "https://example.com/ns/transcode/1";

postproject::Uuid request_proxy(postproject::Production &production,
                                const postproject::Uuid &original_id,
                                const postproject::Uuid &asset_id) {
  const postproject::JobRequest request{"org.postproject:generate-proxy",
                                        {original_id},
                                        asset_id,
                                        postproject::RepresentationKind::proxy,
                                        std::string("proxies")};

  auto transaction = production.beginTransaction();
  const auto job_id = transaction.requestJob(request);
  // Typed job parameters are metadata on the job itself.
  const postproject::ObjectRef job{postproject::ObjectKind::job, job_id};
  transaction.addMetadataValue(
      job, transcode, "profile",
      postproject::MetadataInput::plainString("editing-proxy"));
  transaction.addMetadataValue(
      job, transcode, "max-width",
      postproject::MetadataInput::unsignedInteger(1920));
  transaction.commit();

  const auto requested =
      production.jobs(100, std::nullopt, postproject::JobState::requested,
                      std::string_view("org.postproject:generate-proxy"));
  for (const auto &item : requested.items) {
    std::cout << item.kind << " with " << item.inputs.size()
              << " input(s) into root " << item.target_root.value_or("-")
              << '\n';
  }
  return job_id;
}
// [/request-job]

// [claim-job]
void claim_renew_release(postproject::Production &production,
                         const postproject::Uuid &job_id) {
  auto claim = production.beginTransaction();
  const auto claim_id =
      claim.claimJob(job_id, encoder, worker, t0, t0 + one_minute);
  claim.commit();

  // Renewing and releasing need the claim token returned by claimJob.
  auto renew = production.beginTransaction();
  renew.renewJobClaim(job_id, claim_id, t0 + 30'000'000, t0 + 2 * one_minute);
  renew.commit();

  auto release = production.beginTransaction();
  release.releaseJobClaim(job_id, claim_id); // back to requested
  release.commit();
}
// [/claim-job]

// [complete-job]
postproject::Uuid complete_proxy(postproject::Production &production,
                                 const postproject::Job &job,
                                 const std::string &output_path) {
  const std::int64_t now = t0 + 3 * one_minute;
  auto claim = production.beginTransaction();
  const auto claim_id =
      claim.claimJob(job.id, encoder, worker, now, now + 10 * one_minute);
  claim.commit();

  std::ofstream(output_path, std::ios::binary) << "encoded proxy";

  // Output, provenance, and the job transition commit together or not at all.
  auto transaction = production.beginTransaction();
  const auto proxy_id = transaction.addSingleFileRepresentation(
      job.output_asset_id, job.output_representation_kind, output_path);
  postproject::ActivitySpec activity{};
  activity.kind = job.kind;
  for (const auto &input : job.inputs) {
    activity.inputs.push_back({input, std::nullopt});
  }
  activity.outputs = {{proxy_id, std::nullopt}};
  activity.tool = encoder;
  activity.agent = worker;
  const auto activity_id = transaction.createActivity(activity);
  // Parameters on the activity let the artifact be regenerated later.
  transaction.addMetadataValue(
      {postproject::ObjectKind::activity, activity_id}, transcode, "profile",
      postproject::MetadataInput::plainString("editing-proxy"));
  transaction.completeJob(job.id, claim_id, now + one_minute, proxy_id,
                          activity_id);
  transaction.commit();
  return proxy_id;
}
// [/complete-job]

// [fail-job]
void fail_proxy(postproject::Production &production,
                const postproject::Uuid &job_id) {
  const std::int64_t now = t0 + 5 * one_minute;
  auto claim = production.beginTransaction();
  const auto claim_id =
      claim.claimJob(job_id, encoder, worker, now, now + one_minute);
  claim.commit();

  auto transaction = production.beginTransaction();
  transaction.failJob(job_id, claim_id, now + 1'000'000,
                      "encoder exited with status 1");
  transaction.commit();
}
// [/fail-job]

// [cancel-job]
void cancel(postproject::Production &production,
            const postproject::Uuid &job_id) {
  auto transaction = production.beginTransaction();
  transaction.cancelJob(job_id); // no claim is needed to cancel
  transaction.commit();
}
// [/cancel-job]

// [plan-regeneration]
std::vector<postproject::Uuid>
regenerate(postproject::Production &production,
           const postproject::Uuid &artifact_id) {
  // Planning is read-only: it derives requests from the producing activity.
  const auto plans = production.planRegeneration({artifact_id});

  std::vector<postproject::Uuid> job_ids;
  auto transaction = production.beginTransaction();
  for (const auto &plan : plans) {
    const postproject::JobRequest request{
        plan.job.kind, plan.job.inputs, plan.job.output_asset_id,
        plan.job.output_representation_kind, plan.job.target_root};
    const auto job_id = transaction.requestJob(request);
    for (const auto &parameter : plan.parameters) {
      transaction.addMetadataValue({postproject::ObjectKind::job, job_id},
                                   parameter.vocabulary, parameter.property,
                                   parameter.value);
    }
    job_ids.push_back(job_id);
  }
  transaction.commit();
  return job_ids;
}
// [/plan-regeneration]

} // namespace

int main(int argc, char **argv) {
  if (argc != 2) {
    std::cerr << "usage: postproject-cpp-jobs WORK_DIRECTORY\n";
    return 2;
  }
  const std::filesystem::path work = argv[1];

  try {
    std::filesystem::create_directories(work / "proxies");
    auto production =
        postproject::Production::create((work / "jobs.pproj").string(), "Jobs");
    auto setup = production.beginTransaction();
    const auto asset_id =
        setup.importMedia((work / "rushes" / "A001.mov").string(), "Camera A");
    setup.addMediaRoot("proxies", "Generated proxies");
    setup.commit();
    const auto original_id = production.representations(asset_id).front().id;

    const auto job_id = request_proxy(production, original_id, asset_id);
    auto requested = load_job(production, job_id);
    require(requested.state == postproject::JobState::requested &&
                requested.inputs == std::vector{original_id} &&
                requested.output_asset_id == asset_id &&
                requested.target_root == std::optional<std::string>("proxies"),
            "requested job");
    require(production
                    .queryMetadata(
                        transcode, "max-width",
                        postproject::MetadataInput::unsignedInteger(1920), 10)
                    .items.front()
                    .target ==
                postproject::ObjectRef{postproject::ObjectKind::job, job_id},
            "typed job parameter");

    claim_renew_release(production, job_id);
    const auto released = load_job(production, job_id);
    require(released.state == postproject::JobState::requested &&
                !released.claim.has_value(),
            "released job is requested again");

    const auto proxy_id = complete_proxy(
        production, released, (work / "proxies" / "A001_proxy.mov").string());
    const auto completed = load_job(production, job_id);
    require(completed.state == postproject::JobState::succeeded &&
                completed.completion.has_value() &&
                completed.completion->representation_id == proxy_id,
            "succeeded job");
    const auto producers = production.activitiesProducing(proxy_id);
    require(producers.size() == 1 &&
                producers.front().id == completed.completion->activity_id,
            "job references its activity");

    auto more = production.beginTransaction();
    const postproject::JobRequest thumbnail{
        "org.postproject:generate-thumbnail",
        {original_id},
        asset_id,
        postproject::RepresentationKind::derived,
        std::nullopt};
    const auto failing_id = more.requestJob(thumbnail);
    const auto cancelled_id = more.requestJob(thumbnail);
    more.commit();

    const auto representations_before =
        production.representations(asset_id).size();
    fail_proxy(production, failing_id);
    const auto failed = load_job(production, failing_id);
    require(failed.state == postproject::JobState::failed &&
                failed.failure_diagnostic ==
                    std::optional<std::string>("encoder exited with status 1"),
            "failed job");
    require(production.representations(asset_id).size() ==
                representations_before,
            "failure adds no representation");

    cancel(production, cancelled_id);
    require(load_job(production, cancelled_id).state ==
                postproject::JobState::cancelled,
            "cancelled job");

    const auto before_plan = production.latestRevision()->sequence;
    const auto plans = production.planRegeneration({proxy_id});
    require(production.latestRevision()->sequence == before_plan,
            "planning is read-only");
    require(plans.size() == 1 &&
                plans.front().artifact_representation_id == proxy_id &&
                plans.front().job.kind == "org.postproject:generate-proxy" &&
                plans.front().parameters.size() == 1,
            "regeneration plan");
    const auto regenerated = regenerate(production, proxy_id);
    require(regenerated.size() == 1, "one regeneration job");
    const auto enqueued = load_job(production, regenerated.front());
    require(enqueued.state == postproject::JobState::requested &&
                enqueued.inputs == std::vector{original_id},
            "enqueued regeneration job");
    require(production
                    .queryMetadata(transcode, "profile",
                                   postproject::MetadataInput::plainString(
                                       "editing-proxy"),
                                   10)
                    .items.size() == 3,
            "profile on job, activity, and regeneration job");
  } catch (const std::exception &error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
