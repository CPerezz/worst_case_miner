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

        // Set all mined addresses to 1
        assembly {
            sstore(0xccc8d3967a041bdb4fc6fc426b8b0cc67eff297c, 1)
            sstore(0xc1640c919e9ac2736758776be1af8e93f4e1279c, 1)
            sstore(0xa18fafc0d8719274b012295897b35c222c9584cb, 1)
            sstore(0x42d0736b03118f5ee11001e708332ce6b19070c4, 1)
            sstore(0x1abdb574b762b4eab8e4c1261943cce95f6b2987, 1)
            sstore(0x784ec2f763118ca5bf74afdffcc48f46f70259f7, 1)
            sstore(0x8d857cf4352b6323404f4641902b1317302dcfd9, 1)
            sstore(0xaf8ede15bf83fda6d1a2664271196c1ded9c6969, 1)
            sstore(0x74b71c0d391aedf46f57b213a55a1cf727ba319f, 1)
            sstore(0x2c4283ae4b3b91b2791076bd94f2553cbe0d5589, 1)
            sstore(0x10406a9178c7205b9b9a08763d7b5c32ae0f7714, 1)
            sstore(0x10406a9178c7205b9b9a08763d7b5c32ae0f7714, 1)
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
            sstore(0x10406a9178c7205b9b9a08763d7b5c32ae0f7714, value)
        }
    }

    // Optional: getter to verify the deepest slot value
    function getDeepest() external view returns (uint256 value) {
        assembly {
            value := sload(0x10406a9178c7205b9b9a08763d7b5c32ae0f7714)
        }
    }
}