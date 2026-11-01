import { createMemo, Match, Show, Switch } from "solid-js";
import { DownloadsMenu } from "./components/DownloadsMenu";
import { SearchBox } from "./components/SearchBox";
import { Sidebar } from "./components/Sidebar";
import { TitleBar } from "./components/TitleBar";
import { Toaster } from "./components/Toaster";
import { sessionStore } from "./lib/anitsu";
import { classify, type Place } from "./lib/library";
import { router } from "./lib/router";
import { FolderView } from "./views/FolderView";
import { HomeView } from "./views/HomeView";
import { LoginView } from "./views/LoginView";
import { ShelfView } from "./views/ShelfView";
import { TitleView } from "./views/TitleView";

const only =
  <K extends Place["kind"]>(kind: K) =>
  (p: Place) =>
    p.kind === kind ? (p as Extract<Place, { kind: K }>) : undefined;

function App() {
  const { session, ready } = sessionStore;
  const place = createMemo(() => classify(router.path()));

  return (
    <Show when={ready()} fallback={<div class="h-full bg-bg" />}>
      <Show when={session()} fallback={<LoginView />}>
        <div class="flex h-full flex-col bg-bg">
          <a
            href="#conteudo"
            class="sr-only focus:not-sr-only focus:absolute focus:top-14 focus:left-4 focus:z-50 focus:rounded-lg focus:bg-elevated focus:px-3 focus:py-2"
          >
            Pular para o conteúdo
          </a>
          <TitleBar showNav center={<SearchBox />} actions={<DownloadsMenu />} />
          <div class="flex min-h-0 flex-1">
            <Sidebar />
            <main id="conteudo" class="scrollbar-thin relative min-w-0 flex-1 overflow-x-hidden overflow-y-auto bg-canvas">
              <Switch>
                <Match when={place().kind === "home"}>
                  <HomeView />
                </Match>
                <Match when={only("shelf")(place())} keyed>
                  {(p) => <ShelfView path={p.path} type={p.type} />}
                </Match>
                <Match when={only("title")(place())}>
                  {(p) => <TitleView titlePath={p().titlePath} rest={p().rest} type={p().type} />}
                </Match>
                <Match when={only("folder")(place())} keyed>
                  {(p) => <FolderView path={p.path} />}
                </Match>
              </Switch>
            </main>
          </div>
          <Toaster />
        </div>
      </Show>
    </Show>
  );
}

export default App;
