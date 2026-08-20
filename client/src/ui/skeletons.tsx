import { TopBar } from "./TopBar";

const noop = (): void => {};

function Block({ w, h = 14 }: { w: number | string; h?: number }) {
  return <span className="skeleton" style={{ display: "inline-block", width: w, height: h }} />;
}

/** The projects page while its list loads: the bar stays, the rows shimmer. */
export function ProjectsSkeleton() {
  return (
    <div className="screen">
      <TopBar account={null} onBack={null} onAccount={noop} onSignOut={noop} />
      <main className="screen-main">
        <div className="screen-column">
          <div className="screen-head">
            <div className="screen-title">
              <h1>Projects</h1>
            </div>
            <Block w={370} h={40} />
          </div>
          <ul className="project-rows">
            {[0, 1, 2].map((row) => (
              <li key={row}>
                <div className="project-open">
                  <Block w={18} h={18} />
                  <span className="project-info">
                    <Block w={140 + row * 40} h={15} />
                    <Block w={70} h={11} />
                  </span>
                  <Block w={120} h={13} />
                </div>
              </li>
            ))}
          </ul>
        </div>
      </main>
    </div>
  );
}

/** The account page's section cards, shimmering. */
export function AccountSectionsSkeleton() {
  return (
    <div className="screen-column narrow">
      <div>
        <div className="screen-title">
          <h1>Account</h1>
        </div>
        <Block w={180} h={13} />
      </div>
      {[0, 1, 2].map((section) => (
        <div key={section} className="account-section">
          <Block w={120 + section * 30} h={16} />
          <Block w="100%" h={40} />
          <Block w="100%" h={40} />
        </div>
      ))}
    </div>
  );
}

export function AccountSkeleton() {
  return (
    <div className="screen">
      <TopBar account={null} onBack={null} onAccount={noop} onSignOut={noop} />
      <main className="screen-main">
        <AccountSectionsSkeleton />
      </main>
    </div>
  );
}

/** The editor while the project opens: bar, tree, text and preview placeholders. */
export function EditorSkeleton() {
  return (
    <div className="app">
      <TopBar
        account={null}
        onBack={null}
        onAccount={noop}
        onSignOut={noop}
        context={<Block w={140} h={13} />}
      />
      <main className="panes">
        <nav className="files" style={{ padding: 10, display: "flex", flexDirection: "column", gap: 9 }}>
          {[90, 120, 70, 110, 80].map((width, row) => (
            <Block key={row} w={width} h={12} />
          ))}
        </nav>
        <section className="pane" style={{ padding: 16, gap: 9 }}>
          {[65, 40, 80, 72, 30, 55, 78, 45].map((width, row) => (
            <Block key={row} w={`${width}%`} h={12} />
          ))}
        </section>
        <section className="pane viewer" style={{ alignItems: "center", padding: 24 }}>
          <Block w={420} h={560} />
        </section>
      </main>
    </div>
  );
}
