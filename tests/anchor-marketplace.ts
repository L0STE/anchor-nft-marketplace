import * as anchor from "@coral-xyz/anchor";
import { Program, BN, AnchorError } from "@coral-xyz/anchor";
import { AnchorMarketplace } from "../target/types/anchor_marketplace";
import {
  LAMPORTS_PER_SOL,
  PublicKey,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  Transaction,
  SystemProgram,
} from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  getOrCreateAssociatedTokenAccount,
  createTransferInstruction
} from "@solana/spl-token";
import {
  createNft,
  mplTokenMetadata,
  verifyCollection, 
  deserializeMetadata,
  fetchMetadataFromSeeds,
  findMetadataPda,
} from "@metaplex-foundation/mpl-token-metadata";
import { MPL_TOKEN_METADATA_PROGRAM_ID } from '@metaplex-foundation/mpl-token-metadata';
import { base58 } from "@metaplex-foundation/umi/serializers";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults"
import { 
  createSignerFromKeypair, 
  generateSigner, 
  percentAmount, 
  publicKey, 
  signerIdentity 
} from "@metaplex-foundation/umi";

describe("anchor-marketplace", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.AnchorMarketplace as Program<AnchorMarketplace>;
  const provider = anchor.getProvider();
  const connection = provider.connection;

  let admin = anchor.web3.Keypair.generate();

  const confirm = async (signature: string): Promise<string> => {
    const block = await connection.getLatestBlockhash();
    await connection.confirmTransaction({
      signature,
      ...block,
    });
    return signature;
  };

  const log = async (signature: string): Promise<string> => {
    console.log(
      `Your transaction signature: https://explorer.solana.com/transaction/${signature}?cluster=custom&customUrl=${connection.rpcEndpoint}`
    );
    return signature;
  };

  let marketplacePda: anchor.web3.PublicKey;
  let feeVault: anchor.web3.PublicKey;
  let listingPda: anchor.web3.PublicKey;
  let listingVault: anchor.web3.PublicKey;

  let nftMint: anchor.web3.PublicKey;
  let nftMetadata: anchor.web3.PublicKey;
  let nftMasterEdition: anchor.web3.PublicKey;

  let collectionMint: anchor.web3.PublicKey;
  let collectionMetadata: anchor.web3.PublicKey;
  let collectionMasterEdition: anchor.web3.PublicKey;

  const lister = anchor.web3.Keypair.generate();
  let listerAta: anchor.web3.PublicKey;

  const buyer = anchor.web3.Keypair.generate();
  let buyerAta: anchor.web3.PublicKey;

  it("Airdrop", async () => {
    await connection.requestAirdrop(admin.publicKey, LAMPORTS_PER_SOL * 10).then(confirm).then(log);
    await connection.requestAirdrop(lister.publicKey, LAMPORTS_PER_SOL * 10).then(confirm).then(log);
    await connection.requestAirdrop(buyer.publicKey, LAMPORTS_PER_SOL * 10).then(confirm).then(log);

  })

  it("Creates a new marketplace", async () => {

    const name = "Test Marketplace #2";
    const fee = 0.05;

    marketplacePda = await PublicKey.findProgramAddressSync(([Buffer.from("marketplace"), Buffer.from(name), admin.publicKey.toBuffer()]), program.programId)[0];
    feeVault = await PublicKey.findProgramAddressSync(([Buffer.from("fee_vault"), marketplacePda.toBuffer()]), program.programId)[0];

    const tx = await program.methods
      .initalizeMarketplace(name, fee)
      .accounts({
        admin: admin.publicKey,
        marketplace: marketplacePda,
        feeVault,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([admin]).rpc({skipPreflight: true}).then(confirm).then(log);
  });

  it("Mint Collection NFT", async () => {

    // Metaplex Setup
    const umi = createUmi(connection.rpcEndpoint);
    let umiKeypair = umi.eddsa.createKeypairFromSecretKey(admin.secretKey);
    const signerKeypair = createSignerFromKeypair(umi, umiKeypair);
    const mintSigner = generateSigner(umi);
    umi.use(signerIdentity(signerKeypair));
    umi.use(mplTokenMetadata())

    // Create Collection NFT
    let minttx = createNft(
      umi, 
      {
        mint: mintSigner,
        authority: signerKeypair,
        updateAuthority: umiKeypair.publicKey,
        name: "NFT Example",
        symbol: "EXM",
        uri: "",
        sellerFeeBasisPoints: percentAmount(0),
        creators: [
            {address: umiKeypair.publicKey, verified: true, share: 100 }
        ],
        collection: null,
        uses: null,
        isMutable: true,
        collectionDetails: null,
      }
    );

    const result = await minttx.sendAndConfirm(umi, {
      send: {
        skipPreflight: true
      },
      confirm: {
        commitment: 'confirmed'
      }
    });

    const signature = base58.deserialize(result.signature);
    console.log(`Your transaction signature: https://explorer.solana.com/transaction/${signature[0]}?cluster=custom&customUrl=${connection.rpcEndpoint}`)

    collectionMint = new anchor.web3.PublicKey(mintSigner.publicKey);

    const metadata_seeds = [
      Buffer.from('metadata'),
      new anchor.web3.PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID).toBuffer(),
      new anchor.web3.PublicKey(collectionMint).toBuffer(),
    ];
    collectionMetadata = PublicKey.findProgramAddressSync(metadata_seeds, new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID))[0];

    const master_edition_seeds = [
      ...metadata_seeds,
      Buffer.from("edition")
    ];
    collectionMasterEdition = PublicKey.findProgramAddressSync(master_edition_seeds, new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID))[0]; 
  });

  it("Mint NFT", async () => {

    const umi = createUmi(connection.rpcEndpoint);
    let umiKeypair = umi.eddsa.createKeypairFromSecretKey(lister.secretKey);
    const signerKeypair = createSignerFromKeypair(umi, umiKeypair);
    const mintSigner = generateSigner(umi);
    umi.use(signerIdentity(signerKeypair));
    umi.use(mplTokenMetadata())

    const key = publicKey(collectionMint);

    // Create Collection NFT
    let minttx = createNft(
      umi, 
      {
        mint: mintSigner,
        authority: signerKeypair,
        updateAuthority: umiKeypair.publicKey,
        name: "NFT Example",
        symbol: "EXM",
        uri: "",
        sellerFeeBasisPoints: percentAmount(1),
        creators: [
            {address: umiKeypair.publicKey, verified: true, share: 50 },
            {address: publicKey(admin.publicKey), verified: false, share: 50}
        ],
        collection: {verified: false, key},
        uses: null,
        isMutable: true,
        collectionDetails: null,
      }
    );

    const result = await minttx.sendAndConfirm(umi, {
      send: {
        skipPreflight: true
      },
      confirm: {
        commitment: 'confirmed'
      }
    });

    const signature = base58.deserialize(result.signature);
    console.log(`Your transaction signature: https://explorer.solana.com/transaction/${signature[0]}?cluster=custom&customUrl=${connection.rpcEndpoint}`)

    nftMint = new anchor.web3.PublicKey(mintSigner.publicKey);

    const metadata_seeds = [
      Buffer.from('metadata'),
      new anchor.web3.PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID).toBuffer(),
      new anchor.web3.PublicKey(nftMint).toBuffer(),
    ];
    nftMetadata = PublicKey.findProgramAddressSync(metadata_seeds, new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID))[0];

    const master_edition_seeds = [
      ...metadata_seeds,
      Buffer.from("edition")
    ];
    nftMasterEdition = PublicKey.findProgramAddressSync(master_edition_seeds, new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID))[0]; 
  });

  it("Verify Collection", async () => {

    const umi = createUmi(connection.rpcEndpoint);
    let umiKeypair = umi.eddsa.createKeypairFromSecretKey(admin.secretKey);
    const signerKeypair = createSignerFromKeypair(umi, umiKeypair);
    const mintSigner = generateSigner(umi);
    umi.use(signerIdentity(signerKeypair));
    umi.use(mplTokenMetadata())

    let metadata = publicKey(nftMetadata);
    let collectionMintUmi = publicKey(collectionMint);
    let collection = publicKey(collectionMetadata);
    let collectionMasterEditionAccount = publicKey(collectionMasterEdition);

    let verifyTx = verifyCollection(
      umi,
      {
        metadata,
        collectionAuthority: signerKeypair,
        collectionMint: collectionMintUmi,
        collection,
        collectionMasterEditionAccount,
      }
    );

    const result = await verifyTx.sendAndConfirm(umi, {
      send: {
        skipPreflight: true
      },
      confirm: {
        commitment: 'confirmed'
      }
    });

    const signature = base58.deserialize(result.signature);
    console.log(`Your transaction signature: https://explorer.solana.com/transaction/${signature[0]}?cluster=custom&customUrl=${connection.rpcEndpoint}`)
  });

  it("Creates Listing", async () => {

    listingPda = await PublicKey.findProgramAddressSync(([Buffer.from("listing"), marketplacePda.toBuffer()]), program.programId)[0];
    
    const ata = await getOrCreateAssociatedTokenAccount(connection, lister, nftMint, listingPda, true);
    listingVault = ata.address;

    listerAta = await getAssociatedTokenAddressSync(nftMint, lister.publicKey);

    const price = new BN(1 * LAMPORTS_PER_SOL);

    try {
    const tx = await program.methods
      .list(price)
      .accounts({
        lister: lister.publicKey,
        listerAta,
        marketplace: marketplacePda,
        listing: listingPda,
        collection: collectionMint,
        nft: nftMint,
        metadata: nftMetadata,
        edition: nftMasterEdition,
        sysvarInstruction: SYSVAR_INSTRUCTIONS_PUBKEY,
        tokenMetadataProgram: MPL_TOKEN_METADATA_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([lister]).rpc().then(confirm).then(log);
    } catch (e) {
      console.log(e);
    }
  });

  xit("Delist", async () => {

    const tx = await program.methods
      .delist()
      .accounts({
        lister: lister.publicKey,
        listerAta,
        marketplace: marketplacePda,
        listing: listingPda,
        nft: nftMint,
        metadata: nftMetadata,
        edition: nftMasterEdition,
        sysvarInstruction: SYSVAR_INSTRUCTIONS_PUBKEY,
        tokenMetadataProgram: MPL_TOKEN_METADATA_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([lister]).rpc({skipPreflight: true}).then(confirm).then(log);
  });

  it("Buy", async () => {

    buyerAta = getAssociatedTokenAddressSync(nftMint, buyer.publicKey);
      
    // const metadataInfo = await connection.getAccountInfo(nftMetadata);
    // console.log(deserializeMetadata(metadataInfo.data));

    // const umi = createUmi(connection.rpcEndpoint);
    // const signerKeypair = createSignerFromKeypair(umi, umi.eddsa.createKeypairFromSecretKey(lister.secretKey));
    // umi.use(signerIdentity(signerKeypair));
    // umi.use(mplTokenMetadata())

    // let metadataInfo = await fetchMetadataFromSeeds(umi, {mint: publicKey(nftMint)})
    // console.log(metadataInfo);

    // let metadataInfo = await fetchMetadata(umi, publicKey(nftMetadata));
    // console.log(metadataInfo);

    // For Testing Purposes we know that lister gets 100% of the royalty
    let creator1 = lister.publicKey;
    let share1 = 50;
    let creator2 = admin.publicKey;
    let share2 = 50;
    let seller_fee_basis_points = 100;
    let price = 1 * LAMPORTS_PER_SOL;

    let tx = new Transaction();

    const buyTx = await program.methods
      .buy()
      .accounts({
        buyer: buyer.publicKey,
        lister: lister.publicKey,
        buyerAta,
        listerAta,
        marketplace: marketplacePda,
        feeVault,
        listing: listingPda,
        nft: nftMint,
        metadata: nftMetadata,
        edition: nftMasterEdition,
        sysvarInstruction: SYSVAR_INSTRUCTIONS_PUBKEY,
        tokenMetadataProgram: MPL_TOKEN_METADATA_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      // .signers([buyer]).rpc().then(confirm).then(log);

      .instruction()

      tx.instructions = [
        buyTx,
        SystemProgram.transfer({
          fromPubkey: buyer.publicKey,
          toPubkey: creator1,
          lamports: price * seller_fee_basis_points * share1 / 1000000,
        }),
        SystemProgram.transfer({
          fromPubkey: buyer.publicKey,
          toPubkey: creator2,
          lamports: price * seller_fee_basis_points * share2 / 1000000,
        }),
      ]
      try {
        await provider.sendAndConfirm(tx, [ buyer ]).then(confirm).then(log);
      } catch(e) {
        console.log(e);
        throw(e)
      }
  });

});

