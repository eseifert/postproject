# Assets, representations, and locators

An **asset** is a logical production object. It is not a pathname.

A **representation** is one concrete realization of an asset, such as a camera
original, editorial proxy, mezzanine transcode, or render. An asset may have
multiple representations.

A **locator** describes where or how a representation can currently be
accessed. A representation may have several locators, and moving a file changes
its location rather than its logical identity.

This separation is why PostProject can retain an asset's identity and metadata
when storage paths change.
