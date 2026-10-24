export * from '../api';

export interface RealNote {
  relative_path: string;
  abs_path?: string;
  title: string;
  content: string;
  encoding?: string;
}
