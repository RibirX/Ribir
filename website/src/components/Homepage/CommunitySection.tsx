import React from 'react';
import Link from '@docusaurus/Link';

import {
  DiscordLogoIcon,
  LinkedInLogoIcon,
  TwitterLogoIcon,
} from '@radix-ui/react-icons';

export default function CommunitySection() {
  return (
    <section className="no-underline-links">
      <div className="mx-auto flex w-full flex-col items-center justify-center bg-gradient-to-b from-[#262626] to-black px-4 py-16 pt-64 text-white dark:from-zinc-200/90 dark:to-white dark:text-zinc-700">
        <h2 className="text-3xl">
          Join the <span className="text-primary-100">community</span>
        </h2>
        <p className="mb-10 text-zinc-500">
          Engage with our ever-growing community to get the latest updates,
          product support, and more.
        </p>
        <div className="flex w-full flex-col items-center justify-center gap-2 text-sm font-semibold lg:flex-row lg:gap-8">
          <Link
            className="flex w-full items-center justify-center gap-2 rounded-sm border border-solid border-primary-100 bg-primary-100/10 px-2 py-2 text-primary-100 lg:w-auto"
            href="#"
          >
            <DiscordLogoIcon /> Discord &rarr;
          </Link>
          <Link
            className="flex w-full items-center justify-center gap-2 rounded-sm border border-solid border-primary-100 bg-primary-100/10 px-2 py-2 text-primary-100 lg:w-auto"
            href="https://twitter.com/"
          >
            <TwitterLogoIcon /> Twitter &rarr;
          </Link>
          <Link
            className="flex w-full items-center justify-center gap-2 rounded-sm border border-solid border-primary-100 bg-primary-100/10 px-2 py-2 text-primary-100 lg:w-auto"
            href="#"
          >
            <LinkedInLogoIcon /> LinkedIn &rarr;
          </Link>
        </div>
      </div>
    </section>
  );
}
