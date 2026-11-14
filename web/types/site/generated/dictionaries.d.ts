export interface StagedDictionary {
  name: string;
  file: string;
  sha256: string;
  size: number;
}

export const DICTIONARIES: StagedDictionary[];
