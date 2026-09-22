// Canonical names for the schemas orval names after each operation.

import type {
  AccountShow200User,
  ProjectFileIndex200FilesItem,
  ProjectIndex200ProjectsItem,
} from "./model";

export type Account = AccountShow200User;
export type Project = ProjectIndex200ProjectsItem;
export type ProjectFileInfo = ProjectFileIndex200FilesItem;

/** What someone may do with a project. The API types it as a plain string. */
export type Role = "owner" | "editor" | "viewer";
