import { Request, Response } from 'express';
import { exec } from 'child_process';
import { promisify } from 'util';
import path from 'path';
import {
  getVerificationProof,
  getVerificationStatus,
  revokeVerification,
  submitVerification,
  VerificationLevel,
  VerificationProvider,
} from '../services/verification.service';

const execAsync = promisify(exec);

const VALID_LEVELS: VerificationLevel[] = ['basic', 'enhanced', 'institutional'];
const VALID_PROVIDERS: VerificationProvider[] = ['civic', 'fractal_id', 'mock'];

/**
 * Verify contract against source code
 */
export const verifyContract = async (req: Request, res: Response): Promise<void> => {
  try {
    const { contractId, network = 'testnet' } = req.query;

    if (!contractId || typeof contractId !== 'string') {
      res.status(400).json({
        error: 'contractId parameter is required',
      });
      return;
    }

    // Determine source path based on contract ID
    // This is a simple mapping - in production, this might be stored in DB
    let sourcePath: string;
    if (contractId.startsWith('C') && contractId.length === 56) {
      // For now, assume it's the lending contract
      // In future, could query deployment manifest or database
      sourcePath = path.join(process.cwd(), '../../stellar-lend/contracts/hello-world');
    } else {
      res.status(400).json({
        error: 'Unable to determine source path for contract ID',
      });
      return;
    }

    const scriptPath = path.join(process.cwd(), '../../scripts/verify-contract.sh');
    const command = `${scriptPath} --contract-id ${contractId} --source ${sourcePath} --network ${network}`;

    const { stdout, stderr } = await execAsync(command);

    if (stderr && !stdout.includes('VERIFICATION SUCCESSFUL')) {
      res.status(400).json({
        verified: false,
        error: stderr,
      });
      return;
    }

    res.json({
      verified: true,
      contractId,
      network,
      message: 'Contract verification successful',
    });
  } catch (error) {
    console.error('Verification error:', error);
    res.status(500).json({
      verified: false,
      error: 'Verification failed',
    });
  }
};

export const submitKycVerification = async (req: Request, res: Response): Promise<void> => {
  try {
    const { userAddress, provider = 'mock', level = 'basic', jurisdiction, proofHash } = req.body ?? {};

    if (!userAddress || typeof userAddress !== 'string') {
      res.status(400).json({ error: 'userAddress is required' });
      return;
    }
    if (!VALID_PROVIDERS.includes(provider)) {
      res.status(400).json({ error: 'provider must be civic, fractal_id, or mock' });
      return;
    }
    if (!VALID_LEVELS.includes(level)) {
      res.status(400).json({ error: 'level must be basic, enhanced, or institutional' });
      return;
    }

    const attestation = await submitVerification({
      userAddress,
      provider,
      level,
      jurisdiction,
      proofHash,
    });

    res.status(attestation.status === 'verified' ? 201 : 202).json(attestation);
  } catch (error) {
    console.error('KYC verification error:', error);
    res.status(500).json({ error: 'KYC verification failed' });
  }
};

export const getKycStatus = async (req: Request, res: Response): Promise<void> => {
  try {
    const { userAddress } = req.params;
    const status = await getVerificationStatus(userAddress!);
    if (!status) {
      res.status(404).json({ error: 'Verification attestation not found' });
      return;
    }
    res.json(status);
  } catch (error) {
    console.error('KYC status error:', error);
    res.status(500).json({ error: 'Unable to read verification status' });
  }
};

export const revokeKycAttestation = async (req: Request, res: Response): Promise<void> => {
  try {
    const { userAddress } = req.params;
    const revoked = await revokeVerification(userAddress!);
    if (!revoked) {
      res.status(404).json({ error: 'Verification attestation not found' });
      return;
    }
    res.json(revoked);
  } catch (error) {
    console.error('KYC revoke error:', error);
    res.status(500).json({ error: 'Unable to revoke verification attestation' });
  }
};

export const getPrivacyProof = (req: Request, res: Response): void => {
  const { userAddress } = req.params;
  res.json(getVerificationProof(userAddress!));
};
