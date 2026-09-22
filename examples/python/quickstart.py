"""Create a production and import one opaque media fixture."""

from __future__ import annotations

import argparse
from pathlib import Path

from postproject import OriginIdentity, Production


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("production", type=Path)
    parser.add_argument("media", type=Path)
    args = parser.parse_args()

    with Production.create(args.production, "Python quickstart") as production:
        with production.transaction(
            origin=OriginIdentity("org.postproject:python-quickstart"),
            message="Import quickstart media",
        ) as transaction:
            asset_id = transaction.import_media(
                args.media, display_name="Quickstart media"
            )

        representations = production.representations[asset_id]
        print(f"representations: {len(representations)}")


if __name__ == "__main__":
    main()
