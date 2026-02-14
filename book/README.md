# ZJJ Documentation (mdBook)

This directory contains the Stripe-quality mdBook documentation for ZJJ.

## Building Locally

### Prerequisites

- Rust 1.80+
- mdBook and plugins

### Install Tools

```bash
cargo install mdbook mdbook-mermaid mdbook-toc
```

### Build

```bash
mdbook build
```

### Serve Locally

```bash
mdbook serve --open
```

This will start a local server at `http://localhost:3000` with live-reload.

### Clean

```bash
mdbook clean
```

## Structure

```
book/
├── book.toml           # Configuration
├── src/
│   ├── SUMMARY.md      # Table of contents
│   ├── introduction.md # Landing page
│   ├── quickstart.md   # Quick start guide
│   ├── guide/          # User guide chapters
│   ├── ai/             # AI agent documentation
│   ├── reference/      # Command reference
│   ├── dev/            # Development guide
│   ├── ops/            # Operations guide
│   └── resources/      # FAQ, glossary, etc.
└── theme/
    ├── custom.css      # Custom styling
    └── stripe.css      # Stripe-inspired design
```

## Deployment

The documentation is automatically deployed to GitHub Pages when changes are pushed to `main`.

- **Workflow**: `.github/workflows/deploy-docs.yml`
- **URL**: https://lprior-repo.github.io/zjj/

## Style Guide

### Writing Principles

- **Clear over clever**: Simple, direct language
- **Show, don't tell**: Code examples for everything
- **Progressive disclosure**: Start simple, layer complexity
- **Scannable**: Use headers, lists, tables, callouts

### Code Blocks

Use fenced code blocks with language hints:

\`\`\`bash
zjj add my-feature
\`\`\`

### Callouts

Use styled divs for callouts:

```html
<div class="info">
💡 <strong>Tip</strong>: This is helpful information.
</div>

<div class="warning">
⚠️ <strong>Warning</strong>: This is important to know.
</div>

<div class="note">
✨ <strong>Note</strong>: This is a side note.
</div>
```

### Links

- Internal: Relative links (`./guide/workspaces.md`)
- External: Absolute URLs with descriptive text
- Commands: Link to reference (`[zjj add](./reference/zjj-add.md)`)

## Contributing

When adding new pages:

1. Create the `.md` file in the appropriate directory
2. Add it to `src/SUMMARY.md`
3. Test locally with `mdbook serve`
4. Submit a PR

## Custom Styling

### Colors

Our design system uses Stripe-inspired colors:

- Primary: `#635bff` (Stripe Purple)
- Accent: `#00d4ff` (Stripe Blue)
- Success: `#00d924` (Stripe Green)
- Warning: `#ff5a00` (Stripe Orange)

### Components

See `theme/custom.css` for available components:

- `.hero` - Landing page hero section
- `.features` - Feature grid
- `.quickstart-cards` - Quick start cards
- `.command-example` - Terminal examples
- `.api-section` - API reference sections
- `.status-badge` - Status indicators

## Plugins

### Mermaid

For diagrams:

\`\`\`mermaid
graph LR
    A[Main] --> B[Workspace]
    B --> C[Done]
\`\`\`

### Table of Contents

Automatically generated in each chapter.

## Support

For questions or issues with the documentation:

- Open an issue: https://github.com/lprior-repo/zjj/issues
- Discussion: https://github.com/lprior-repo/zjj/discussions
