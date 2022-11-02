import React from 'react';
import {Avatar, Box, Group, Text, ThemeIcon, UnstyledButton, useMantineTheme} from '@mantine/core';
import {useProfile} from "../ProfileProvider";
import {MdAccountCircle, MdArrowRight} from "react-icons/md";
import Link from "next/link";
import {useUser} from "@supabase/auth-helpers-react";


export default function UserProfile() {
    const theme = useMantineTheme()
    const user = useUser()
    const profileData = useProfile()

    return <Link href='/profile' passHref={true}>
        <Box
            sx={{
                paddingTop: theme.spacing.sm,
                borderTop: `1px solid ${
                    theme.colorScheme === 'dark' ? theme.colors.dark[4] : theme.colors.gray[2]
                }`,
            }}
        >
            <UnstyledButton
                sx={{
                    display: 'block',
                    width: '100%',
                    padding: theme.spacing.xs,
                    borderRadius: theme.radius.sm,
                    color: theme.colorScheme === 'dark' ? theme.colors.dark[0] : theme.black,

                    '&:hover': {
                        backgroundColor:
                            theme.colorScheme === 'dark' ? theme.colors.dark[6] : theme.colors.gray[0],
                    },
                }}
            >
                <Group>
                    {profileData.profile?.name &&
                        <Avatar
                            src={`https://ui-avatars.com/api/?name=${profileData.profile?.name}&background=random&rounded=true`}
                            radius="xl"
                        />
                    }

                    {profileData.profile == null &&
                        <ThemeIcon radius={"xl"} variant={'outline'} color={'purple'} size={42} p={2}>
                            <MdAccountCircle size={40}/>
                        </ThemeIcon>
                    }

                    <Box sx={{flex: 1}}>

                        <Text size="sm" weight={500}>
                            {profileData.profile ? (profileData.profile?.name || "Profil") : "Logare"}
                        </Text>
                        {user?.email &&
                            <Text color="dimmed" size="xs">
                                {user?.email}
                            </Text>
                        }
                    </Box>

                    <MdArrowRight size={18}/>
                </Group>
            </UnstyledButton>
        </Box>
    </Link>
}
