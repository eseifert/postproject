# Guide for application users

PostProject is normally embedded inside another application. It remembers the
difference between the media you are working with and the path where one copy
currently lives.

That distinction allows an application to recognize moved media using stored
file evidence. If several candidates are equally credible, PostProject reports
the ambiguity instead of guessing; the application should show the choices and
let you confirm one.

Originals, proxies, optimized media, and generated results can all be separate
representations of a logical asset. Future provenance records explain how a
result was produced without claiming that the result is cryptographically
trusted.

A project file stores identity, representation structure, resource locators,
fingerprint evidence, and project metadata. PostProject does not upload this
data or contact identifier registries. There is no network service in the
current release.
