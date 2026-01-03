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
            sstore(0x2edf0d839937169ca51e3c549be1bf966a664161, 1)
            sstore(0xe1633467289d77c130c7d27ed858518ab7d0c6b4, 1)
            sstore(0x4b8412523ff7f4c9b5f1403dc0f64234adeedaeb, 1)
            sstore(0xd2523db496ee5d757d818825fda8c64f12f079ca, 1)
            sstore(0x0981bd09c433f8027fb6810b22bed91d02567e6d, 1)
            sstore(0x96c817570a5e31fce868f2642f10d563ab4ad987, 1)
            sstore(0x1c4bdda8ded59ac242e66d1f4a189da65b3ee6cd, 1)
            sstore(0x293db26a7df9301aefe515e840957dbe1ae19ce5, 1)
            sstore(0x3b5cbf64b9440c1dbc5b74a098229a8aebc97094, 1)
            sstore(0x5773e4125df9f78c75e5f9a415570c4f3a3e7731, 1)
            sstore(0xcfbfb64c2124c23c81c7c4e34ae37ab41c8315ed, 1)
            sstore(0xcfbfb64c2124c23c81c7c4e34ae37ab41c8315ed, 1)
            sstore(0xcfbfb64c2124c23c81c7c4e34ae37ab41c8315ed, 1)
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
            sstore(0xcfbfb64c2124c23c81c7c4e34ae37ab41c8315ed, value)
        }
    }

    // Optional: getter to verify the deepest slot value
    function getDeepest() external view returns (uint256 value) {
        assembly {
            value := sload(0xcfbfb64c2124c23c81c7c4e34ae37ab41c8315ed)
        }
    }
}