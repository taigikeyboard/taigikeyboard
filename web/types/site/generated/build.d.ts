export interface StagedBuild {
  /** The full commit hash the site was staged from. */
  commit: string;
  /** `commit` abbreviated as git does (`%h`). */
  short: string;
  /** When the site was staged, ISO 8601 in UTC. */
  time: string;
}

export const STAGED: StagedBuild;
