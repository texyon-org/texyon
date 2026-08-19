import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";

import { EditorScreen } from "../EditorScreen";
import { SAMPLE } from "../sample";
import { Workspace } from "../workspace/Workspace";

export function SampleScreen() {
  // Fresh per visit, nothing is saved.
  const [workspace] = useState(() => new Workspace(SAMPLE));
  const navigate = useNavigate();

  return (
    <EditorScreen
      key="sample"
      workspace={workspace}
      primary="main.tex"
      title="Sample"
      projectId={null}
      onBack={() => void navigate({ to: "/" })}
    />
  );
}
