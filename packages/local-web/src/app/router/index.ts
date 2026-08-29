import { createRouter } from '@tanstack/react-router';
import { routeTree } from '@web/routeTree.gen';

const appBasePath = import.meta.env.VITE_APP_BASE_PATH ?? '/';

export const router = createRouter({
  routeTree,
  basepath: appBasePath,
});

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
