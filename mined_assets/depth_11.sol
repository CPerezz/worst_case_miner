// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract WorstCaseERC20 {
    // ERC20 State
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;
    uint256 public totalSupply;

    // Token metadata - returning constants to save gas
    string public constant name = "WorstCase";
    string public constant symbol = "WORST";
    uint8 public constant decimals = 18;

    constructor() {
        // Mint total supply to deployer
        totalSupply = 1_000_000_000 * 10 ** 18; // 1 billion tokens
        balanceOf[msg.sender] = totalSupply;

        // Set all mined storage keys to 1
        assembly {
            sstore(0xcf6e875ee1d81ccfc22f6c16c036eeb571fc7c3252d9493d6bdf129c195367f4, 1)
            sstore(0x519944d997555aeb30554920a0995386a52d8d1c47bc80fdc601359e617a9551, 1)
            sstore(0x505e19da53fe2989b5704b397c655df71f27e2f4a36176cd4045030309679cba, 1)
            sstore(0x50d9d86e91a99978c55a09079f520f18f62e5769e6c8fabed47bbbbe64c1c1aa, 1)
            sstore(0x50d422c65782bcd9ac8c6ffa846a4822268930698522520e503ea1d250f4a923, 1)
            sstore(0x50d45ce85383a0a6cb4209ca15b0475f5f8a14fb30f1a32e6b375254c873c199, 1)
            sstore(0x50d454b8030e195e2d4e262d40d752a997a1efb3f6e354e7d827b81498b85055, 1)
            sstore(0x50d454580f0a23e1d207180f2a9303280585c02dbae23048c247fe8e8b05239b, 1)
            sstore(0x50d45454f42d0024f2a83dd0263e7719cd303106ef6e0675b36a6334a0749ccd, 1)
            sstore(0x50d454540e1c46c424899a5c8a5065b5a654ef55c39395ccadf71ee34ffe27d6, 1)
            sstore(0x50d45454068830eb9f9e626bd9ef329cc24b396e62d65ce0911fb8d759d3d9ef, 1)
        }
    }

    // Minimal ERC20 implementation
    function transfer(address to, uint256 amount) public returns (bool) {
        require(balanceOf[msg.sender] >= amount, "Insufficient balance");
        balanceOf[msg.sender] -= amount;
        balanceOf[to] += amount;
        return true;
    }

    function approve(address spender, uint256 amount) public returns (bool) {
        allowance[msg.sender][spender] = amount;
        return true;
    }

    function transferFrom(
        address from,
        address to,
        uint256 amount
    ) public returns (bool) {
        require(balanceOf[from] >= amount, "Insufficient balance");
        require(
            allowance[from][msg.sender] >= amount,
            "Insufficient allowance"
        );

        balanceOf[from] -= amount;
        balanceOf[to] += amount;
        allowance[from][msg.sender] -= amount;

        return true;
    }

    // Attack method - writes to the deepest storage slot
    function attack(uint256 value) external {
        assembly {
            sstore(0x50d45454068830eb9f9e626bd9ef329cc24b396e62d65ce0911fb8d759d3d9ef, value)
        }
    }

    // Optional: getter to verify the deepest slot value
    function getDeepest() external view returns (uint256 value) {
        assembly {
            value := sload(0x50d45454068830eb9f9e626bd9ef329cc24b396e62d65ce0911fb8d759d3d9ef)
        }
    }
}