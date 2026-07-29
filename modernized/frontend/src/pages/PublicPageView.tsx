// Public site-page viewer (item 3): renders an active `type='other'` page served
// by GET /api/pages/:slug at the PUBLIC route /pages/:slug (no auth guard). Shows
// the page name as a title and the admin-authored body below it. A 404 renders a
// dedicated not-found state; other failures render a generic error with a retry.
//
// @implements FS-033: public serving of a custom `other` page by slug.
import { useEffect } from "react";
import { observer } from "mobx-react-lite";
import { Alert, Box, Button, CircularProgress, Stack, Typography } from "@mui/material";
import { useParams } from "react-router-dom";
import { useStores } from "../stores/StoreContext";

export const PublicPageView = observer(function PublicPageView() {
  const { publicPage } = useStores();
  const { slug } = useParams();

  useEffect(() => {
    if (slug) void publicPage.load(slug);
  }, [publicPage, slug]);

  if (publicPage.loading) {
    return (
      <Box sx={{ display: "flex", justifyContent: "center", py: 8 }}>
        <CircularProgress />
      </Box>
    );
  }

  if (publicPage.notFound) {
    return (
      <Alert severity="warning" data-testid="page-not-found">
        This page could not be found.
      </Alert>
    );
  }

  if (publicPage.error) {
    return (
      <Box>
        <Alert severity="error" sx={{ mb: 2 }}>
          {publicPage.error}
        </Alert>
        <Button variant="outlined" onClick={() => slug && void publicPage.load(slug)}>
          Retry
        </Button>
      </Box>
    );
  }

  if (!publicPage.page) return null;

  return (
    <Stack spacing={3}>
      <Typography variant="h4" component="h1">
        {publicPage.page.name}
      </Typography>
      {/*
        The page body is admin-authored HTML from a trusted author (only an
        admin can create/edit site pages via the admin panel), so it is rendered
        as HTML. There is no untrusted user input on this path.
      */}
      <Box
        data-testid="page-body"
        sx={{ "& img": { maxWidth: "100%" } }}
        dangerouslySetInnerHTML={{ __html: publicPage.page.body }}
      />
    </Stack>
  );
});
