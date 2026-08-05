import type { RepoPackChonkifyAdapter } from "./repoPackCompression";
import type { RepoContextPack } from "./repoIntelligence";

const GENERATED_PATH = /(?:^|\/)(?:dist|build|coverage|node_modules|target)\//i;

function compressFileContent(content: string, maxLines = 24) {
  const lines = content.split(/\r?\n/);
  if (lines.length <= maxLines) {
    return { text: lines.join("\n"), startLine: 1, endLine: lines.length };
  }
  const head = Math.max(4, Math.floor(maxLines * 0.6));
  const tail = Math.max(2, maxLines - head);
  return {
    text: [
      ...lines.slice(0, head),
      "... [chonkify omitted middle lines] ...",
      ...lines.slice(-tail),
    ].join("\n"),
    startLine: 1,
    endLine: lines.length,
  };
}

function syntheticFileContent(path: string, estimatedTokens: number): string {
  const lineCount = Math.max(8, Math.ceil(estimatedTokens / 12));
  return Array.from({ length: lineCount }, (_, index) => `// ${path}:${index + 1}`).join(
    "\n",
  );
}

export const chonkifyRepoPackAdapter: RepoPackChonkifyAdapter<RepoContextPack> = {
  name: "switchboard-chonkify",
  version: "1.0.0",
  compress({ currentPack, files }) {
    const sourceSpans: Array<{
      repositoryRelativePath: string;
      startLine: number;
      endLine: number;
    }> = [];
    const skippedFiles: Array<{ repositoryRelativePath: string; reason: string }> = [];
    const compressedFiles = [];

    for (const file of [...files].sort((left, right) =>
      left.repositoryRelativePath.localeCompare(right.repositoryRelativePath),
    )) {
      if (GENERATED_PATH.test(file.repositoryRelativePath)) {
        skippedFiles.push({
          repositoryRelativePath: file.repositoryRelativePath,
          reason: "generated or dependency path",
        });
        continue;
      }
      const body =
        file.content ||
        syntheticFileContent(
          file.repositoryRelativePath,
          currentPack.files.find((entry) => entry.path === file.repositoryRelativePath)
            ?.estimatedTokens ?? 120,
        );
      const result = compressFileContent(body);
      const original = currentPack.files.find(
        (entry) => entry.path === file.repositoryRelativePath,
      );
      sourceSpans.push({
        repositoryRelativePath: file.repositoryRelativePath,
        startLine: result.startLine,
        endLine: result.endLine,
      });
      compressedFiles.push({
        ...(original ?? {
          path: file.repositoryRelativePath,
          role: "unknown" as const,
          language: "unknown",
          includeByDefault: true,
          reasons: ["chonkify compressed"],
        }),
        estimatedTokens: Math.max(1, Math.ceil(result.text.length / 4)),
      });
    }

    const estimatedTokens = compressedFiles.reduce(
      (total, file) => total + file.estimatedTokens,
      0,
    );

    return {
      pack: {
        ...currentPack,
        files: compressedFiles,
        estimatedTokens,
      },
      sourceSpans,
      skippedFiles,
    };
  },
};

export function repoPackSourceFilesFromContextPack(
  pack: RepoContextPack,
): Array<{ repositoryRelativePath: string; content: string }> {
  return pack.files.map((file) => ({
    repositoryRelativePath: file.path,
    content: syntheticFileContent(file.path, file.estimatedTokens),
  }));
}
