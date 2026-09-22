# Image sequences and spanned media

Some media is useful only as a set of files. A folder of numbered EXR frames is
one shot, and a long camera recording may be split across several files. An
application using PostProject can keep each set as one representation instead
of making every file look like unrelated media.

For an image sequence, PostProject remembers the filename pattern, first and
last frame, frame spacing, number padding, playback rate, and any gaps already
known at import. It does not need to create a separate database object for
every frame.

Availability describes whether the complete representation can be used:

- **Online** means every required member is currently present.
- **Partial** means some required members are present and others are missing.
- **Offline** means no required content can currently be reached.
- **Ambiguous** means several plausible replacements need a person to choose.
- **Error** means the check could not finish safely, with an explanation.

For a sequence, a partial result includes the missing frame numbers. The
application decides whether to stop, hold another frame, show black, or use a
different policy; PostProject reports the facts and does not silently choose a
playback workaround.

Spanned recordings preserve the order of their parts. Package-like media can
also include named members such as essence, metadata, indexes, sidecars, or
thumbnails. A missing required member affects availability. A missing optional
member is still reported but does not make otherwise usable media partial.

Moving the folder or files changes their storage locators, not the identity of
the asset or representation. Stored content evidence can help an application
find the moved media. If several candidates are equally credible, the
application should show them and ask for an explicit confirmation.
