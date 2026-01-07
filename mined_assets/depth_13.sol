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
            sstore(0x7fdf85d87765cac19aaf50fb4403d9d531353394, 1)
            sstore(0xae6feb06701f3dc26bc4f73c3510426fb4bb8b0c, 1)
            sstore(0x0e6ad12829f1cfb80ccfcaf5be0e0eb794b1b5f6, 1)
            sstore(0xd322b17ad7ccc924ab4227ca5a9a809321e55f9f, 1)
            sstore(0xc05ce9f2146d858edf3ef883dd413543503a74cf, 1)
            sstore(0x687bdef5fe34c06e6c2553da518fb279753e20e6, 1)
            sstore(0x706c093f1f82ebbbd453d170d19f0f18a8ee0ed7, 1)
            sstore(0x01d2c4d612287ddb1c80f50a705be62f07b29228, 1)
            sstore(0xe255bdb57f2e476ad4804e5f5188800c647c8f0b, 1)
            sstore(0x9f286f304ed95ead84bce63e12870209c9849224, 1)
            sstore(0x9f286f304ed95ead84bce63e12870209c9849224, 1)
            sstore(0x9f286f304ed95ead84bce63e12870209c9849224, 1)
            sstore(0x9f286f304ed95ead84bce63e12870209c9849224, 1)
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
            sstore(0x9f286f304ed95ead84bce63e12870209c9849224, value)
        }
    }

    // Optional: getter to verify the deepest slot value
    function getDeepest() external view returns (uint256 value) {
        assembly {
            value := sload(0x9f286f304ed95ead84bce63e12870209c9849224)
        }
    }
}