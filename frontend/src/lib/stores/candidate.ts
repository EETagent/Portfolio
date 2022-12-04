import { writable } from 'svelte/store';

export interface CandidateData {
	name?: string;
	surname?: string;
	birthplace?: string;
	birthdate?: string;
	address?: string;
	telephone?: string;
	citizenship?: string;
	email?: string;
	sex?: string;
	study?: string;
	personalIdNumber?: string;
	parentName?: string;
	parentSurname?: string;
	parentTelephone?: string;
	parentEmail?: string;
}

export interface CandidatePreview {
	applicationId: number;
	name: string;
	surname: string;
	study: string;
}

export interface CandidateLogin {
	applicationId: number;
	password: string;
}

export interface CreateCandidate {
	applicationId: number;
	personalIdNumber: string;
}

export interface CreateCandidateLogin extends CreateCandidate {
	password: string;
}

export const candidateData = writable<CandidateData>({});
