# TieZ Theme Package v1

TieZ theme packages are ZIP archives with the `.zip` or `.tiez-theme` extension.
All files must live at the archive root.

```text
theme.zip
├── metadata.json
├── theme.css
├── preview-light.png
└── preview-dark.png
```

## `metadata.json`

```json
{
  "apiVersion": 1,
  "id": "retro",
  "name": {
    "zh": "复古",
    "en": "Retro",
    "tw": "復古"
  },
  "description": {
    "zh": "经典机械式立体界面",
    "en": "A mechanical retro interface"
  },
  "category": "retro",
  "version": "1.0.0",
  "minAppVersion": "1.0.0",
  "supportsCustomBackground": false,
  "supportsSurfaceOpacity": false,
  "windowMaterial": "none"
}
```

Rules:

- `apiVersion` must be `1`.
- `id` is 3–50 lowercase ASCII letters, numbers, or hyphens. It must not begin with `store-`.
- at least one of `name.zh` and `name.en` is required.
- `category` is one of `nature`, `minimal`, `retro`, `dark`, `cute`, or `other`.
- `windowMaterial` is `none`, `mica`, or `acrylic`. Community themes should use `none`; native materials remain an app-controlled capability.

The installed runtime id is derived by TieZ as `store-<id>`.

## `theme.css`

The stylesheet must include the installed runtime selector. For a package whose
metadata id is `retro`, the selector is `.theme-store-retro`.

Theme CSS builds on the semantic variables and component structure shipped by
TieZ. A package therefore declares `minAppVersion` and does not copy the shared
base/component CSS into the archive.

For v1:

- maximum CSS size is 100 KiB;
- `@import` is forbidden;
- remote, local-file, and Tauri asset URLs are forbidden;
- inline `data:` images and fragment URLs are allowed;
- JavaScript is never loaded from a theme package.

## Preview images

Both previews are required. They must be PNG, JPEG, or WebP images, no larger
than 800×600 pixels and 500 KiB each.

## Package limits

- maximum archive size: 5 MiB;
- maximum uncompressed payload: 2 MiB;
- maximum archive entries: 16;
- nested paths and unknown files are rejected.

