import {
  createRootRoute,
  createRoute,
  createRouter,
  Link,
  redirect,
} from "@tanstack/react-router";
import type { ErrorComponentProps } from "@tanstack/react-router";

import {
  getAccountShowQueryOptions,
  getProjectIndexQueryOptions,
  inviteShow,
  projectShow,
} from "./api/generated";
import { ApiError } from "./api/http";
import { queryClient } from "./queryClient";
import { AccountScreen } from "./session/AccountScreen";
import { InviteScreen } from "./session/InviteScreen";
import { openProject } from "./session/load";
import { ProjectEditorScreen } from "./session/ProjectEditorScreen";
import { ProjectsScreen } from "./session/ProjectsScreen";
import { SampleScreen } from "./session/SampleScreen";
import { SignInScreen } from "./session/SignInScreen";
import { AccountSkeleton, EditorSkeleton, ProjectsSkeleton } from "./ui/skeletons";

/** 401 means go sign in, and come back here afterwards when `back` says where here is. */
function gate(failure: unknown, back?: string): never {
  if (failure instanceof ApiError && failure.isUnauthenticated) {
    throw redirect({ to: "/sign-in", search: back === undefined ? {} : { back } });
  }
  throw failure;
}

const rootRoute = createRootRoute();

const projectsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  // Listing projects is also the signed-in check.
  loader: () => queryClient.ensureQueryData(getProjectIndexQueryOptions()).catch(gate),
  pendingMs: 0,
  pendingComponent: ProjectsSkeleton,
  errorComponent: ({ error }: ErrorComponentProps) => (
    <SignInScreen
      notice={`The server could not be reached (${error.message}). You can still try the sample.`}
    />
  ),
  component: ProjectsScreen,
});

const editorRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/projects/$projectId",
  params: {
    parse: ({ projectId }) => {
      const id = Number(projectId);
      if (!Number.isInteger(id) || id <= 0) throw new Error(`not a project id: ${projectId}`);
      return { projectId: id };
    },
    stringify: ({ projectId }) => ({ projectId: String(projectId) }),
  },
  // A fresh workspace on every entry.
  gcTime: 0,
  loader: async ({ params }) => {
    try {
      const { project } = await projectShow(params.projectId);
      const workspace = await openProject(project);
      return { project, workspace };
    } catch (failure) {
      gate(failure);
    }
  },
  pendingMs: 0,
  pendingComponent: EditorSkeleton,
  errorComponent: ({ error }: ErrorComponentProps) => (
    <div className="gate">
      <div className="card">
        <h1>Could not open this project</h1>
        <div className="card-error">{error.message}</div>
        <Link className="card-link" to="/">
          Back to projects
        </Link>
      </div>
    </div>
  ),
  component: ProjectEditorScreen,
});

const accountRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/account",
  loader: () => queryClient.ensureQueryData(getAccountShowQueryOptions()).catch(gate),
  pendingMs: 0,
  pendingComponent: AccountSkeleton,
  component: AccountScreen,
});

const sampleRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/sample",
  component: SampleScreen,
});

const inviteRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/invite/$token",
  loader: ({ params }) =>
    inviteShow(params.token).catch((failure: unknown) => gate(failure, `/invite/${params.token}`)),
  pendingMs: 0,
  pendingComponent: ProjectsSkeleton,
  errorComponent: ({ error }: ErrorComponentProps) => (
    <div className="gate">
      <div className="card">
        <h1>This invite does not work</h1>
        <div className="card-error">{error.message}</div>
        <p className="card-sub">It may have been revoked. Ask whoever sent it for a new one.</p>
        <Link className="card-link" to="/">
          Back to projects
        </Link>
      </div>
    </div>
  ),
  component: InviteScreen,
});

const signInRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/sign-in",
  // Only a path on this site, so a crafted link cannot bounce someone elsewhere.
  validateSearch: (search: Record<string, unknown>): { back?: string } =>
    typeof search.back === "string" && search.back.startsWith("/") && !search.back.startsWith("//")
      ? { back: search.back }
      : {},
  component: SignInScreen,
});

const routeTree = rootRoute.addChildren([
  projectsRoute,
  editorRoute,
  accountRoute,
  sampleRoute,
  inviteRoute,
  signInRoute,
]);

export const router = createRouter({
  routeTree,
  defaultNotFoundComponent: () => (
    <div className="gate">
      <div className="card">
        <h1>Nothing here</h1>
        <Link className="card-link" to="/">
          Back to projects
        </Link>
      </div>
    </div>
  ),
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
