module.exports = {
  "disableEmoji": false,
  "list": [
    "test",
    "feat",
    "fix",
    "build",
    "docs",
    "refactor",
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
    "algo",
    "widgets",
    "ribir",
    "theme",
    "geom",
    "examples",
    "deps",
    "dev-helper",
    "tools"
  ],
  "types": {
    "build": {
      "description": "Build process or auxiliary tool changes",
      "emoji": "🤖",
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
    "test": {
      "description": "Adding missing tests or correcting existing tests",
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