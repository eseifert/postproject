//! Deliberately small built-in metadata catalog.

use super::{
    MetadataCardinality, MetadataPropertyAlias, MetadataPropertyDefinition,
    MetadataVocabularyDefinition,
};
use crate::MetadataValueKind;

/// IPTC Video Metadata Hub 1.7 JSON schema identity.
pub const IPTC_VMH_JSON_VOCABULARY: &str =
    "https://iptc.org/std/videometadatahub/recommendation/iptc-vmhub-1.7-schema.json";
/// Dublin Core elements namespace used by XMP and other mappings.
pub const DUBLIN_CORE_ELEMENTS_VOCABULARY: &str = "http://purl.org/dc/elements/1.1/";
/// Adobe XMP Basic namespace.
pub const XMP_BASIC_VOCABULARY: &str = "http://ns.adobe.com/xap/1.0/";
/// Current `EBUCore` namespace identity.
pub const EBUCORE_VOCABULARY: &str = "urn:ebu:metadata-schema:ebucore";
/// Namespace reserved for the small set of PostProject-owned metadata terms.
pub const POSTPROJECT_METADATA_VOCABULARY: &str = "https://postproject.org/ns/metadata/";

const TEXT: &[MetadataValueKind] = &[MetadataValueKind::String, MetadataValueKind::LangString];
const STRING: &[MetadataValueKind] = &[MetadataValueKind::String];
const TIMESTAMP: &[MetadataValueKind] = &[MetadataValueKind::Timestamp];

const TITLE_ALIASES: &[MetadataPropertyAlias] =
    &[alias("XMP", "dc:title"), alias("EBUCore", "title/dc:title")];
const KEYWORD_ALIASES: &[MetadataPropertyAlias] = &[
    alias("XMP", "dc:subject"),
    alias("EBUCore", "description/dc:description"),
];

const IPTC_VMH_PROPERTIES: &[MetadataPropertyDefinition] = &[
    property(
        "title",
        "Title",
        "A short title for the video.",
        TEXT,
        MetadataCardinality::Single,
        TITLE_ALIASES,
    ),
    property(
        "keywords",
        "Keywords",
        "Free-choice phrases describing what the video is about.",
        TEXT,
        MetadataCardinality::Repeatable,
        KEYWORD_ALIASES,
    ),
];

const DUBLIN_CORE_PROPERTIES: &[MetadataPropertyDefinition] = &[
    property(
        "title",
        "Title",
        "A name given to the resource.",
        TEXT,
        MetadataCardinality::Repeatable,
        &[],
    ),
    property(
        "subject",
        "Subject",
        "A topic of the resource.",
        TEXT,
        MetadataCardinality::Repeatable,
        &[],
    ),
];

const XMP_BASIC_PROPERTIES: &[MetadataPropertyDefinition] = &[
    property(
        "CreateDate",
        "Create date",
        "The date and time the resource was originally created.",
        TIMESTAMP,
        MetadataCardinality::Single,
        &[],
    ),
    property(
        "CreatorTool",
        "Creator tool",
        "The first known tool used to create the resource.",
        STRING,
        MetadataCardinality::Single,
        &[],
    ),
];

/// Small built-in registry. Unknown vocabularies and properties remain valid.
pub const METADATA_VOCABULARIES: &[MetadataVocabularyDefinition] = &[
    vocabulary(
        IPTC_VMH_JSON_VOCABULARY,
        "IPTC Video Metadata Hub 1.7 JSON",
        IPTC_VMH_JSON_VOCABULARY,
        IPTC_VMH_PROPERTIES,
    ),
    vocabulary(
        DUBLIN_CORE_ELEMENTS_VOCABULARY,
        "Dublin Core elements",
        "https://www.dublincore.org/specifications/dublin-core/dces/",
        DUBLIN_CORE_PROPERTIES,
    ),
    vocabulary(
        XMP_BASIC_VOCABULARY,
        "XMP Basic",
        "https://developer.adobe.com/xmp/docs/xmp-namespaces/xmp/",
        XMP_BASIC_PROPERTIES,
    ),
    vocabulary(
        EBUCORE_VOCABULARY,
        "EBUCore",
        "https://tech.ebu.ch/publications/tech3293",
        &[],
    ),
    vocabulary(
        POSTPROJECT_METADATA_VOCABULARY,
        "PostProject metadata",
        POSTPROJECT_METADATA_VOCABULARY,
        &[],
    ),
];

const fn alias(profile: &'static str, property: &'static str) -> MetadataPropertyAlias {
    MetadataPropertyAlias { profile, property }
}

const fn property(
    property: &'static str,
    label: &'static str,
    description: &'static str,
    accepted_kinds: &'static [MetadataValueKind],
    cardinality: MetadataCardinality,
    aliases: &'static [MetadataPropertyAlias],
) -> MetadataPropertyDefinition {
    MetadataPropertyDefinition {
        property,
        label,
        description,
        accepted_kinds,
        cardinality,
        aliases,
        validator: None,
    }
}

const fn vocabulary(
    vocabulary: &'static str,
    label: &'static str,
    reference: &'static str,
    properties: &'static [MetadataPropertyDefinition],
) -> MetadataVocabularyDefinition {
    MetadataVocabularyDefinition {
        vocabulary,
        label,
        reference,
        properties,
    }
}
