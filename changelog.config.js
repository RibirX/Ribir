module.exports = {
  "disableEmoji": false,
  "list": [
    "test",
    "feat",
    "fix",
    "build",
    "ci",
    "docs",
    "refactor",
    "release",
    "perf",
    "ce"
  ],
  "maxMessageLength": 96,
  "minMessageLength": 3,
  "questions": [
    "type",
    "scope",
    "subject",
    "body",
    "breaking",
    "issues",
    "lerna"
  ],
  "scopes": [
    "core",
    "painter",
    "macros",
    "gpu",
    "text",
    "algo",
    "widgets",
    "ribir",
    "theme",
    "geom",
    "examples"
  ],
  "types": {
    "chore": {
      "description": "Build process or auxiliary tool changes",
      "emoji": "🤖",
      "value": "chore"
    },
    "ci": {
      "description": "CI related changes",
      "emoji": "🎡",
      "value": "ci"
    },
    "build": {
      "description": "Changes that affect the build system or external dependencies",
      "emoji": "🎡",
      "value": "build"
    },
    "docs": {
      "description": "Documentation only changes",
      "emoji": "✏️",
      "value": "docs"
    },
    "feat": {
      "description": "A new feature",
      "emoji": "🎸",
      "value": "feat"
    },
    "fix": {
      "description": "A bug fix",
      "emoji": "🐛",
      "value": "fix"
    },
    "perf": {
      "description": "A code change that improves performance",
      "emoji": "⚡️",
      "value": "perf"
    },
    "refactor": {
      "description": "A code change that neither fixes a bug or adds a feature",
      "emoji": "💡",
      "value": "refactor"
    },
    "release": {
      "description": "Create a release commit",
      "emoji": "🏹",
      "value": "release"
    },
    "test": {
      "description": "Adding missing tests",
      "emoji": "💍",
      "value": "test"
    },
    "ce": {
      "description": "improve the compile error of macros",
      "emoji": "🔧",
      "value": "ce"
    }

  }
};