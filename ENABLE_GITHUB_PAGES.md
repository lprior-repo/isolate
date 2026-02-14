# Enable GitHub Pages for ZJJ Documentation

The documentation site is ready! Follow these steps to enable GitHub Pages and make it live.

## Steps to Enable

1. **Go to your repository settings**
   - Navigate to: https://github.com/lprior-repo/zjj/settings/pages

2. **Configure Source**
   - Under "Build and deployment"
   - Source: Select **"GitHub Actions"**
   - Click **Save**

3. **Wait for deployment**
   - Go to the Actions tab: https://github.com/lprior-repo/zjj/actions
   - You should see the "Deploy Documentation" workflow running
   - Wait for it to complete (usually 2-3 minutes)

4. **Visit your documentation site**
   - Your site will be live at: **https://lprior-repo.github.io/zjj/**

## Verification

Once deployed, verify the site works:

1. Visit https://lprior-repo.github.io/zjj/
2. Check the landing page loads correctly
3. Test navigation and search
4. Verify all pages are accessible

## Troubleshooting

### Workflow doesn't run automatically

If the GitHub Actions workflow doesn't trigger:

1. Go to Actions tab
2. Select "Deploy Documentation" workflow
3. Click "Run workflow" manually
4. Select branch: `main`
5. Click "Run workflow"

### Pages not enabled

If you see "GitHub Pages is currently disabled":

1. Go to Settings → Pages
2. Enable Pages
3. Select Source: "GitHub Actions"

### 404 errors

If you see 404 errors:

1. Check the Actions tab for deployment status
2. Ensure the workflow completed successfully
3. Wait 5-10 minutes for DNS propagation

### Permission errors

If you see permission errors in Actions:

1. Go to Settings → Actions → General
2. Scroll to "Workflow permissions"
3. Select "Read and write permissions"
4. Check "Allow GitHub Actions to create and approve pull requests"
5. Click "Save"

## Auto-Deploy

The documentation auto-deploys on every push to `main` that changes:
- `book/**` - mdBook source files
- `docs/**` - Markdown documentation
- `.github/workflows/deploy-docs.yml` - Deployment workflow

## Manual Build

To build and preview locally:

```bash
cd book
mdbook serve --open
```

This starts a local server at http://localhost:3000

## Custom Domain (Optional)

To use a custom domain:

1. Add a `CNAME` file to `book/book/` with your domain
2. Configure DNS with your domain registrar
3. Update `site-url` in `book/book.toml`

## Next Steps

Once GitHub Pages is enabled and the site is live:

1. ✅ Share the documentation URL
2. ✅ Update README badges if needed
3. ✅ Add link to documentation in repository description
4. ✅ Announce the new documentation to users

---

**Documentation URL**: https://lprior-repo.github.io/zjj/

The site features:
- Stripe-quality design
- Full-text search
- Responsive layout
- Dark/light themes
- Fast navigation
- Print-friendly
