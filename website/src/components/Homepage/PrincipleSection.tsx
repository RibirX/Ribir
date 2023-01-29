import Link from '@docusaurus/Link';
import React from 'react';
import PRINCIPLES from '../../principles';

export default function PrincipleSection() {
  return (
    <section className="my-20 px-6">
      <div className="mx-auto max-w-5xl">
        <span className="ribir-badge">Principle</span>
        <h2 className="lg:text-3xl">Want to know more?</h2>
        <p className="text-text-400">
          Learn Ribir's principles quickly <br />
        </p>

        <div className="no-underline-links mt-10 grid grid-cols-1 gap-12 md:grid-cols-2 lg:grid-cols-3">
          {PRINCIPLES.map((principle) => (
            <div
              className="group flex flex-col justify-between"
              key={principle.title}
            >
              <div>
                <h3 className="font-semibold text-black group-hover:text-primary dark:text-white dark:group-hover:text-primary-100 lg:text-xl">
                  {principle.title}
                </h3>
                <p className="leading-snug text-text-400">
                  {principle.description}
                </p>
              </div>
            </div>
          ))}
        </div>

        <div className="my-20 flex flex-wrap items-center justify-center gap-3 text-center">
          <span>View all</span>
          <div className="flex gap-2">
            <Link className="underline underline-offset-8" href="/docs/introduction">
              Docs
            </Link>
            <Link
              className="underline underline-offset-8"
              href="https://ribir.org/blog"
              target="_blank"
            >
              Blogs
            </Link>
          </div>
        </div>
      </div>
    </section>
  );
}
