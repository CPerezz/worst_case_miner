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
            sstore(0x2f0d0760931190beeb72941883cfad91a464458fe43beb5f084caef9d22aa342, 1)
            sstore(0x7e2ccba5b0e583ffc5e247fd2e316f3557abcb7d53099a0d0546f163ee019deb, 1)
            sstore(0x7c37b46d0932bb65a1f0ae0b230bcc8248843e37c2bcba4bd2ab09e159076145, 1)
            sstore(0x7c85d9bc0fe927ce979d0ecb3fc86c3ced2ba43b5831cd1335f5722b194b1d84, 1)
            sstore(0x7c87e0b3a4209a6f5f336d5f4a97175ec925deee9d51a5943fc48ae9344bd05d, 1)
            sstore(0x7c877239ed3a76fbdec569acde22f06749ba6e9f10f04064f43ff2eb274aeb03, 1)
            sstore(0x7c877518b497b39b8fe96b9eaa3dc372312b3d96c6384e881e72b537f4bc8bd6, 1)
            sstore(0x7c8775026dcd84023c86108d2fa80f3414cc790b77f46a461b459471687b8501, 1)
            sstore(0x7c877509dbcb23d80feabcc88b2e85e547e0b98c1631d6c3e08eab7e84464f6f, 1)
            sstore(0x7c8775092f8d878e47a61dd1c723d51b258c8d1908f3f1d5238607c23b094cb3, 1)
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
            sstore(0x7c8775092f8d878e47a61dd1c723d51b258c8d1908f3f1d5238607c23b094cb3, value)
        }
    }

    // Optional: getter to verify the deepest slot value
    function getDeepest() external view returns (uint256 value) {
        assembly {
            value := sload(0x7c8775092f8d878e47a61dd1c723d51b258c8d1908f3f1d5238607c23b094cb3)
        }
    }
}