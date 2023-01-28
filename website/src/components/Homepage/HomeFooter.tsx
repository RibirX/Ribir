import React from 'react';
import Link from '@docusaurus/Link';
import { useColorMode } from '@docusaurus/theme-common';

import {
  DiscordLogoIcon,
  LinkedInLogoIcon,
  TwitterLogoIcon,
} from '@radix-ui/react-icons';
import clsx from 'clsx';

export default function HomeFooter() {
  const { colorMode } = useColorMode();

  return (
    <footer className="bg-secondary-900">
      <div
        className={clsx('mx-auto flex max-w-7xl flex-col gap-4 px-10 py-8 lg:flex-row lg:items-center lg:gap-8')}
      >
        <div>
          <img src={`/logo/${colorMode}.svg`} alt="Logo" className="h-10" />
        </div>
        <div className="flex items-center gap-3">
          <Link href="https://community.dyte.io">
            <DiscordLogoIcon className="h-6 w-6 text-zinc-400 hover:text-primary" />
          </Link>
          <Link href="https://twitter.com/dyte_io">
            <TwitterLogoIcon className="h-6 w-6 text-zinc-400 hover:text-primary" />
          </Link>
          <Link href="https://linkedin.com/company/dyteio">
            <LinkedInLogoIcon className="h-6 w-6 text-zinc-400 hover:text-primary" />
          </Link>
        </div>
        <div className="flex-1 text-zinc-400 lg:text-right">
          Copyright &copy; Dyte since 2020. All rights reserved.
        </div>
      </div>
    </footer>
  );
}
