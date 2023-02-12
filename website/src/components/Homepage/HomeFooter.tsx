import React from 'react';
import { useColorMode } from '@docusaurus/theme-common';
import clsx from 'clsx';

export default function HomeFooter() {
  const { colorMode } = useColorMode();

  return (
    <footer className="bg-secondary-900">
      <div
        className={clsx('mx-auto flex max-w-7xl flex-col gap-4 px-10 py-8 lg:flex-row lg:items-center lg:gap-8')}
      >
        <div className="flex-1 text-zinc-400 lg:text-center">
          <p>
            The official website uses <a target={"_blank"} href={"https://docs.dyte.io/"}>dyte.io</a> theme, thanks for dyte.io.
            <br/>
            Build with RibirX and <span className="emoji">❤️</span>
          </p>
        </div>
      </div>
    </footer>
  );
}
